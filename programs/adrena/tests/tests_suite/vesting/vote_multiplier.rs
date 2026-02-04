use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddVestParams, BucketName},
        state::{cortex::Cortex, vest::Vest},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
    spl_governance::state::token_owner_record::TokenOwnerRecordV2,
};

const USDC_DECIMALS: u8 = 6;

pub async fn vote_multiplier() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice1",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "alice2",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "alice3",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "alice4",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "alice5",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "alice6",
                token_balances: hashmap! {},
            },
        ],
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
            payer_user_name: "alice1",
        }],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    //
    // Multipliers should work
    //
    {
        let multipliers = [
            10_000, // x1 (MIN)
            10_100, // x1.01
            20_010, // x2.001
            30_001, // x3.0001
            40_000, // x4 (MAX)
        ];

        for (i, vote_multiplier) in multipliers.iter().enumerate() {
            let name = format!("alice{}", i + 1);
            let user = test_setup.get_user_keypair_by_name(&name);

            test_instructions::init_user_staking(
                &test_setup.program_test_ctx,
                user,
                &test_setup.payer_keypair,
                &lm_token_mint_pda,
                &test_setup.pool_pda,
            )
            .await
            .unwrap();

            let current_time =
                utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

            let (vest_pda, _) = test_instructions::add_vest(
                &test_setup.program_test_ctx,
                &test_setup.admin_keypair,
                &test_setup.payer_keypair,
                user,
                &AddVestParams {
                    amount: utils::scale(1_000_000, Cortex::LM_DECIMALS),
                    origin_bucket: BucketName::CoreContributor.into(),
                    unlock_start_timestamp: current_time,
                    unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
                    vote_multiplier: *vote_multiplier,
                },
            )
            .await
            .unwrap();

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            let vest_account =
                utils::get_account::<Vest>(&test_setup.program_test_ctx, vest_pda).await;

            // Check the voting power in the governance is the correct one
            assert_eq!(vest_account.vote_multiplier, *vote_multiplier);

            let governance_governing_token_owner_record_pda =
                pda::get_governance_governing_token_owner_record_pda(
                    &test_setup.governance_realm_pda,
                    &governance_token_mint_pda,
                    &user.pubkey(),
                );

            // Check the amount in the governance to match the multiplier
            let governing_token_owner_record = utils::get_borsh_account::<TokenOwnerRecordV2>(
                &test_setup.program_test_ctx,
                &governance_governing_token_owner_record_pda,
            )
            .await;

            assert_eq!(
                governing_token_owner_record.governing_token_deposit_amount,
                utils::scale(1_000_000, Cortex::LM_DECIMALS) * *vote_multiplier as u64
                    / Cortex::BPS_POWER as u64
            );
        }
    }

    //
    // Multiplier should fail
    //
    {
        {
            let vote_multiplier = 9_999; // Less than MIN (1x)
            let user = test_setup.get_user_keypair_by_name("alice6");

            let current_time =
                utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

            assert!(test_instructions::add_vest(
                &test_setup.program_test_ctx,
                &test_setup.admin_keypair,
                &test_setup.payer_keypair,
                user,
                &AddVestParams {
                    amount: utils::scale(1_000_000, Cortex::LM_DECIMALS),
                    origin_bucket: BucketName::CoreContributor.into(),
                    unlock_start_timestamp: current_time,
                    unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
                    vote_multiplier,
                },
            )
            .await
            .is_err());
        }

        {
            let vote_multiplier = 40_001; // More than MAX (4x)
            let user = test_setup.get_user_keypair_by_name("alice6");

            let current_time =
                utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

            assert!(test_instructions::add_vest(
                &test_setup.program_test_ctx,
                &test_setup.admin_keypair,
                &test_setup.payer_keypair,
                user,
                &AddVestParams {
                    amount: utils::scale(1_000_000, Cortex::LM_DECIMALS),
                    origin_bucket: BucketName::CoreContributor.into(),
                    unlock_start_timestamp: current_time,
                    unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
                    vote_multiplier,
                },
            )
            .await
            .is_err());
        }
    }
}
