use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddVestParams, BucketName, MintStakedLmTokensFromBucketParams},
        state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
    spl_governance::state::token_owner_record::TokenOwnerRecordV2,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn mint_staked_lm_tokens_from_bucket() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {
                "usdc" => utils::scale(100_000, USDC_DECIMALS),
                "eth" => utils::scale(50, ETH_DECIMALS),
            },
        }],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
            },
            utils::MintParam {
                name: "eth",
                decimals: ETH_DECIMALS,
            },
        ],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(10_000_000, Cortex::USD_DECIMALS),
        vec![
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "usdc",
                    is_stable: true,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1, Cortex::PRICE_DECIMALS),
                        initial_conf: 10000000, // 10 bps
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(0, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1500, Cortex::PRICE_DECIMALS),
                        initial_conf: 15000000000, // 10 bps
                        oracle_name: LimitedString::new("eth"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::ETH,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(0, ETH_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(360000000, Cortex::LM_DECIMALS),
        utils::scale(90000000, Cortex::LM_DECIMALS),
        utils::scale(550000000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    test_instructions::init_user_staking(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &lm_token_mint_pda,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    test_instructions::mint_staked_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintStakedLmTokensFromBucketParams {
            bucket_name: BucketName::CoreContributor.into(),
            amount: utils::scale(180000000, Cortex::LM_DECIMALS),
            reason: "Mint 50% of core contributor bucket allocation".to_string(),
            locked_days: 540,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    test_instructions::sync_user_voting_power(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check the governance account and voting power
    {
        let cortex_pda = pda::get_cortex_pda().0;
        let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

        let cortex_account =
            utils::get_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        let governance_governing_token_owner_record_pda =
            pda::get_governance_governing_token_owner_record_pda(
                &cortex_account.governance_realm,
                &governance_token_mint_pda,
                &alice.pubkey(),
            );

        let token_owner_record = utils::get_borsh_account::<TokenOwnerRecordV2>(
            &test_setup.program_test_ctx,
            &governance_governing_token_owner_record_pda,
        )
        .await;

        assert_eq!(
            token_owner_record.governing_token_deposit_amount,
            // x4 staking voting power
            utils::scale(180000000 * 4, Cortex::GOVERNANCE_SHADOW_TOKEN_DECIMALS),
        );
    }

    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

    test_instructions::add_vest(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        alice,
        &AddVestParams {
            amount: utils::scale(100000, Cortex::LM_DECIMALS),
            origin_bucket: BucketName::CoreContributor.into(),
            unlock_start_timestamp: current_time,
            unlock_end_timestamp: current_time + utils::days_in_seconds(7),
            vote_multiplier: Cortex::BPS_POWER as u32 * 2, // x2
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    test_instructions::sync_user_voting_power(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check the governance account and voting power
    {
        let cortex_pda = pda::get_cortex_pda().0;
        let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

        let cortex_account =
            utils::get_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        let governance_governing_token_owner_record_pda =
            pda::get_governance_governing_token_owner_record_pda(
                &cortex_account.governance_realm,
                &governance_token_mint_pda,
                &alice.pubkey(),
            );

        let token_owner_record = utils::get_borsh_account::<TokenOwnerRecordV2>(
            &test_setup.program_test_ctx,
            &governance_governing_token_owner_record_pda,
        )
        .await;

        assert_eq!(
            token_owner_record.governing_token_deposit_amount,
            // x4 staking voting power + x2 vest voting power
            utils::scale(
                180000000 * 4 + 100000 * 2,
                Cortex::GOVERNANCE_SHADOW_TOKEN_DECIMALS
            ),
        );
    }
}
