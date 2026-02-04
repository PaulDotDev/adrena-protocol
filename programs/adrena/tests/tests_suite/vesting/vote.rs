use {
    crate::{
        adapters, test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddVestParams, BucketName},
        state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;

pub async fn vote() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {},
        }],
        vec![utils::MintParam {
            name: "usdc",
            decimals: USDC_DECIMALS,
        }],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(10_000_000, Cortex::USD_DECIMALS),
        vec![utils::SetupCustodyWithLiquidityParams {
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
        }],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let alice = test_setup.get_user_keypair_by_name("alice");

    let governance_token_mint_pda = utils::pda::get_governance_token_mint_pda().0;

    test_instructions::init_user_staking(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &lm_token_mint_pda,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    // Alice: vest 1m token, unlock period from now to in 7 days
    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

    test_instructions::add_vest(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        alice,
        &AddVestParams {
            amount: utils::scale(1_000_000, Cortex::LM_DECIMALS),
            origin_bucket: BucketName::CoreContributor.into(),
            unlock_start_timestamp: current_time,
            unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
            vote_multiplier: Cortex::BPS_POWER as u32, // x1
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let governance_pda = adapters::spl_governance::create_governance(
        &test_setup.program_test_ctx,
        &alice.pubkey(),
        alice,
        &test_setup.payer_keypair,
        &test_setup.governance_realm_pda,
        &governance_token_mint_pda,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap()
    .0;

    let proposal_pda = adapters::spl_governance::create_proposal(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        "Test Proposal".to_string(),
        "Description".to_string(),
        &test_setup.governance_realm_pda,
        &governance_pda,
        &governance_token_mint_pda,
        alice,
        alice,
    )
    .await
    .unwrap();

    adapters::spl_governance::cast_vote(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.governance_realm_pda,
        &governance_pda,
        &proposal_pda,
        &governance_token_mint_pda,
        &alice.pubkey(),
        &alice.pubkey(),
        alice,
        true,
    )
    .await
    .unwrap();

    adapters::spl_governance::cancel_proposal(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.governance_realm_pda,
        &governance_pda,
        &proposal_pda,
        &governance_token_mint_pda,
        &alice.pubkey(),
        alice,
    )
    .await
    .unwrap();

    adapters::spl_governance::relinquish_vote(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.governance_realm_pda,
        &governance_pda,
        &proposal_pda,
        &governance_token_mint_pda,
        &alice.pubkey(),
        alice,
    )
    .await
    .unwrap();

    // Alice: claim vest
    test_instructions::claim_vest(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        alice,
        &test_setup.governance_realm_pda,
    )
    .await
    .unwrap();
}
