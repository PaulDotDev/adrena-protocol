use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidStakeParams, AddLiquidityParams, AddVestParams, BucketName, ClaimStakesParams,
            RemoveLiquidStakeParams,
        },
        state::{cortex::Cortex, staking::StakingRound},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signature::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn liquid_staking_overlap_party() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
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
                        initial_conf: 0,
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
                        initial_conf: 0,
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
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");

    let cortex_stake_reward_mint = test_setup.get_fee_redistribution_mint();

    let eth_mint = &test_setup.get_mint_by_name("eth");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    {
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();
    }

    // Prep work: Alice get 100 governance tokens using vesting
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            alice,
            &AddVestParams {
                amount: utils::scale(100, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        // Move until vest end
        utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    // Alice stake
    {
        // ROUNDS
        //
        //   prev    current    next
        // [      ] [       ] [   1   ]
        //
        // Overlap of 0
        test_instructions::add_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLiquidStakeParams {
                amount: utils::scale(1, Cortex::LM_DECIMALS),
            },
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // ROUNDS
        //
        //   prev    current    next
        // [      ] [       ] [   11   ]
        //
        // Overlap of 0
        test_instructions::add_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLiquidStakeParams {
                amount: utils::scale(10, Cortex::LM_DECIMALS),
            },
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // ROUNDS
        //
        //   prev    current     next
        // [      ] [       ] [   5   ]
        //
        // Overlap of 0
        test_instructions::remove_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            RemoveLiquidStakeParams {
                amount: utils::scale(6, Cortex::LM_DECIMALS),
            },
            &cortex_stake_reward_mint,
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Move to next round
    {
        // ROUNDS
        //
        //   prev    current     next
        // [      ] [   5   ] [   5   ]
        //
        // Overlap of 0

        // Then

        // ROUNDS
        //
        //   prev    current     next
        // [   5   ] [   5   ] [   5   ]
        //
        // Overlap of 0
        for _ in 0..2 {
            {
                test_instructions::add_liquidity(
                    &test_setup.program_test_ctx,
                    martin,
                    &test_setup.payer_keypair,
                    &test_setup.pool_pda,
                    eth_mint,
                    AddLiquidityParams {
                        amount_in: 250000000,
                        min_lp_amount_out: 1,
                        oracle_prices: Some(
                            utils::get_oracle_prices_as_chaos_labs_bundle(
                                &test_setup.program_test_ctx,
                            )
                            .await,
                        ),
                    },
                )
                .await
                .unwrap();
            }

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

            test_instructions::add_liquid_stake(
                &test_setup.program_test_ctx,
                alice,
                &test_setup.payer_keypair,
                AddLiquidStakeParams {
                    amount: utils::scale(1, Cortex::LM_DECIMALS),
                },
                &test_setup.governance_realm_pda,
                &test_setup.pool_pda,
                &lm_token_mint_pda,
            )
            .await
            .unwrap();
        }
    }

    // Claim stakes
    test_instructions::claim_stakes(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &alice.pubkey(),
        &test_setup.pool_pda,
        &lm_token_mint_pda,
        &ClaimStakesParams {
            locked_stake_indexes: None,
        },
    )
    .await
    .unwrap();
}
