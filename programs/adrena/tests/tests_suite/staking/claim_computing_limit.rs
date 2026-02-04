use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidityParams, AddLockedStakeParams, AddVestParams, BucketName, ClaimStakesParams,
        },
        state::{cortex::Cortex, staking::StakingRound},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn claim_computing_limit() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(500000, USDC_DECIMALS),
                    "eth" => utils::scale(50, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(500000, USDC_DECIMALS),
                    "eth" => utils::scale(50, ETH_DECIMALS),
                },
            },
        ],
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
                liquidity_amount: utils::scale(1_500, USDC_DECIMALS),
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
                liquidity_amount: utils::scale(1, ETH_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");

    let eth_mint = test_setup.get_mint_by_name("eth");
    let usdc_mint = test_setup.get_mint_by_name("usdc");

    let alice_usdc_ata = utils::find_associated_token_account(&alice.pubkey(), &usdc_mint).0;

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let users = [alice, martin];

    {
        for user in users {
            test_instructions::init_user_staking(
                &test_setup.program_test_ctx,
                user,
                &test_setup.payer_keypair,
                &lm_token_mint_pda,
                &test_setup.pool_pda,
            )
            .await
            .unwrap();
        }
    }

    // Prep work: Alice & Martin get 5000 governance tokens using vesting
    {
        for user in users {
            let current_time =
                utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

            test_instructions::add_vest(
                &test_setup.program_test_ctx,
                &test_setup.admin_keypair,
                &test_setup.payer_keypair,
                user,
                &AddVestParams {
                    amount: utils::scale(5000, Cortex::LM_DECIMALS),
                    origin_bucket: BucketName::CoreContributor.into(),
                    unlock_start_timestamp: current_time,
                    unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                    vote_multiplier: Cortex::BPS_POWER as u32, // x1
                },
            )
            .await
            .unwrap();
        }

        // Move until vest end
        utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

        for user in users {
            test_instructions::claim_vest(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                user,
                &test_setup.governance_realm_pda,
            )
            .await
            .unwrap();
        }
    }

    // Maximize the number of locked stakes
    for _ in 0..adrena::state::user_staking::MAX_LOCKED_STAKE_COUNT {
        for user in users {
            test_instructions::add_locked_stake(
                &test_setup.program_test_ctx,
                user,
                &test_setup.payer_keypair,
                AddLockedStakeParams {
                    amount: utils::scale(10, Cortex::LM_DECIMALS),
                    locked_days: 540,
                },
                &lm_token_mint_pda,
                &test_setup.governance_realm_pda,
            )
            .await
            .unwrap();
        }

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    for _ in 0..adrena::state::staking::MAX_RESOLVED_ROUNDS {
        // Generate platform activity to fill current round' rewards
        {
            test_instructions::add_liquidity(
                &test_setup.program_test_ctx,
                alice,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                &eth_mint,
                AddLiquidityParams {
                    amount_in: 1000000,
                    min_lp_amount_out: 1,
                    oracle_prices: Some(
                        utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                            .await,
                    ),
                },
            )
            .await
            .unwrap();
        }

        // Resolve current round
        {
            utils::warp_forward(
                &test_setup.program_test_ctx,
                StakingRound::ROUND_MIN_DURATION_SECONDS,
            )
            .await;

            test_instructions::resolve_staking_round(
                &test_setup.program_test_ctx,
                alice,
                &test_setup.payer_keypair,
                &lm_token_mint_pda,
            )
            .await
            .unwrap();

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;
        }
    }

    let alice_usdc_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

    // Users should be able to get theirs rewards for all rounds for half theirs locked stakes
    for user in users {
        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            user,
            &test_setup.payer_keypair,
            &user.pubkey(),
            &test_setup.pool_pda,
            &lm_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: Some(vec![
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                ]),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    let alice_usdc_balance_after_1 =
        utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

    // Alice should claim USDC
    assert_eq!(
        alice_usdc_balance_after_1 - alice_usdc_balance_before,
        161568
    );

    // Users should be able to get theirs rewards for all rounds for half theirs locked stakes
    for user in users {
        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            user,
            &test_setup.payer_keypair,
            &user.pubkey(),
            &test_setup.pool_pda,
            &lm_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: Some(vec![
                    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
                ]),
            },
        )
        .await
        .unwrap();
    }

    let alice_usdc_balance_after_2 =
        utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

    // Alice should claim USDC
    assert_eq!(
        alice_usdc_balance_after_2 - alice_usdc_balance_after_1,
        142560
    );
}
