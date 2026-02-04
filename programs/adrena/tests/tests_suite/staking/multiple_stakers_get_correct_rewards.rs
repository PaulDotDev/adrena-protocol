use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidStakeParams, AddLiquidityParams, AddLockedStakeParams, AddVestParams,
            BucketName, ClaimStakesParams, SwapParams,
        },
        state::{
            cortex::Cortex,
            staking::{Staking, StakingRound},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn multiple_stakers_get_correct_rewards() {
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
            utils::UserParam {
                name: "paul",
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
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        Some("alice"),
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");
    let paul = test_setup.get_user_keypair_by_name("paul");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    let cortex_stake_reward_mint = test_setup.get_fee_redistribution_mint();

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    // Prep work
    let users = [alice, martin, paul];

    // Init LM + LP staking for alice/martin/paul
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

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    {
        // alice/martin/paul get 2 governance tokens using vesting
        {
            let current_time =
                utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

            for user in users.into_iter() {
                test_instructions::add_vest(
                    &test_setup.program_test_ctx,
                    &test_setup.admin_keypair,
                    &test_setup.payer_keypair,
                    user,
                    &AddVestParams {
                        amount: utils::scale(2, Cortex::LM_DECIMALS),
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

            for user in users.into_iter() {
                println!("Claim vest for {}", user.pubkey());
                test_instructions::claim_vest(
                    &test_setup.program_test_ctx,
                    &test_setup.payer_keypair,
                    user,
                    &test_setup.governance_realm_pda,
                )
                .await
                .unwrap();

                utils::warp_forward(&test_setup.program_test_ctx, 1).await;
            }
        }

        // Add liquidity with martin/paul to get LP tokens
        {
            test_instructions::add_liquidity(
                &test_setup.program_test_ctx,
                martin,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                eth_mint,
                AddLiquidityParams {
                    amount_in: utils::scale(1, ETH_DECIMALS),
                    min_lp_amount_out: 1,
                    oracle_prices: Some(
                        utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                            .await,
                    ),
                },
            )
            .await
            .unwrap();

            test_instructions::add_liquidity(
                &test_setup.program_test_ctx,
                paul,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                eth_mint,
                AddLiquidityParams {
                    amount_in: utils::scale(1, ETH_DECIMALS),
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
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Find out all accounts
    let alice_lm_token_account =
        utils::find_associated_token_account(&alice.pubkey(), &lm_token_mint_pda).0;
    let paul_lm_token_account =
        utils::find_associated_token_account(&paul.pubkey(), &lm_token_mint_pda).0;
    let martin_lm_token_account =
        utils::find_associated_token_account(&martin.pubkey(), &lm_token_mint_pda).0;

    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lm_staking_staked_token_vault_pda =
        utils::get_staking_staked_token_vault_pda(&lm_staking_pda).0;
    let lm_staking_reward_token_vault_pda =
        utils::get_staking_reward_token_vault_pda(&lm_staking_pda).0;
    let lm_staking_lm_reward_token_vault_pda =
        utils::get_staking_lm_reward_token_vault_pda(&lm_staking_pda).0;

    // Check initial values
    {
        assert_eq!(
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_lm_token_account)
                .await,
            utils::scale(2, Cortex::LM_DECIMALS)
        );
        assert_eq!(
            utils::get_token_account_balance(&test_setup.program_test_ctx, paul_lm_token_account)
                .await,
            utils::scale(2, Cortex::LM_DECIMALS)
        );
        assert_eq!(
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_lm_token_account)
                .await,
            utils::scale(2, Cortex::LM_DECIMALS)
        );

        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_staked_token_vault_pda
            )
            .await,
            0
        );
        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_reward_token_vault_pda
            )
            .await,
            1200000
        );
        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_lm_reward_token_vault_pda
            )
            .await,
            0
        );
    }

    // Add staking for alice/martin/paul with different amounts & time
    {
        // Stake LM tokens
        {
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

            test_instructions::add_locked_stake(
                &test_setup.program_test_ctx,
                martin,
                &test_setup.payer_keypair,
                AddLockedStakeParams {
                    amount: 1500000,
                    locked_days: 180,
                },
                &lm_token_mint_pda,
                &test_setup.governance_realm_pda,
            )
            .await
            .unwrap();

            test_instructions::add_locked_stake(
                &test_setup.program_test_ctx,
                paul,
                &test_setup.payer_keypair,
                AddLockedStakeParams {
                    amount: utils::scale(1, Cortex::LM_DECIMALS),
                    locked_days: 360,
                },
                &lm_token_mint_pda,
                &test_setup.governance_realm_pda,
            )
            .await
            .unwrap();
        }
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check staking accounts
    {
        let lm_staking =
            utils::get_account::<Staking>(&test_setup.program_test_ctx, lm_staking_pda).await;

        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_staked_token_vault_pda
            )
            .await,
            3500000
        );
        assert_eq!(lm_staking.nb_locked_tokens, 2500000);
        assert_eq!(lm_staking.nb_liquid_tokens, 1000000);
        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_reward_token_vault_pda
            )
            .await,
            1200000
        );
        assert_eq!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                lm_staking_lm_reward_token_vault_pda
            )
            .await,
            0
        );

        assert_eq!(lm_staking.resolved_lm_reward_token_amount, 0);
        assert_eq!(lm_staking.resolved_lm_staked_token_amount, 0);
        assert_eq!(lm_staking.resolved_reward_token_amount, 0);
        assert_eq!(lm_staking.resolved_staked_token_amount, 0);
        assert_eq!(
            lm_staking.resolved_staking_rounds.len(),
            StakingRound::MAX_RESOLVED_ROUNDS
        );

        // Nothing for the actual round as we just staked
        assert_eq!(lm_staking.current_staking_round.lm_total_stake, 0);
        assert_eq!(lm_staking.current_staking_round.total_stake, 0);

        assert_eq!(lm_staking.next_staking_round.lm_total_stake, 5125000);
        assert_eq!(lm_staking.next_staking_round.total_stake, 8000000);
    }

    // warp to the next round and resolve the current one
    // this round bear no rewards for the new staking
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

    // Check staking rounds
    {
        let lm_staking =
            utils::get_account::<Staking>(&test_setup.program_test_ctx, lm_staking_pda).await;

        assert_eq!(lm_staking.resolved_lm_reward_token_amount, 0);
        assert_eq!(lm_staking.resolved_lm_staked_token_amount, 0);
        assert_eq!(lm_staking.resolved_reward_token_amount, 0);
        assert_eq!(lm_staking.resolved_staked_token_amount, 0);
        assert_eq!(
            lm_staking.resolved_staking_rounds.len(),
            StakingRound::MAX_RESOLVED_ROUNDS
        );

        assert_eq!(lm_staking.current_staking_round.total_stake, 8000000);
        assert_eq!(lm_staking.current_staking_round.lm_total_stake, 5125000);

        assert_eq!(lm_staking.next_staking_round.total_stake, 8000000);
        assert_eq!(lm_staking.next_staking_round.lm_total_stake, 5125000);
    }

    // Swap to generate rewards also for the LP stakers
    {
        test_instructions::swap(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            eth_mint,
            SwapParams {
                // 0.5 ETH
                amount_in: 500000000,
                min_amount_out: 1,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();
    }

    // warp to the next round and resolve the current one
    // this round bear rewards for the new staking
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
    }

    // Check staking rounds
    {
        {
            let lm_staking =
                utils::get_account::<Staking>(&test_setup.program_test_ctx, lm_staking_pda).await;

            assert_eq!(lm_staking.resolved_lm_reward_token_amount, 143333333332);
            assert_eq!(lm_staking.resolved_lm_staked_token_amount, 5125000);
            assert_eq!(lm_staking.resolved_reward_token_amount, 1200000);
            assert_eq!(lm_staking.resolved_staked_token_amount, 8000000);
            assert_eq!(
                lm_staking.resolved_staking_rounds.len(),
                StakingRound::MAX_RESOLVED_ROUNDS
            );

            assert_eq!(lm_staking.current_staking_round.total_stake, 8000000);
            assert_eq!(lm_staking.current_staking_round.lm_total_stake, 5125000);

            assert_eq!(lm_staking.next_staking_round.total_stake, 8000000);
            assert_eq!(lm_staking.next_staking_round.lm_total_stake, 5125000);

            assert_eq!(
                utils::get_token_account_balance(
                    &test_setup.program_test_ctx,
                    lm_staking_reward_token_vault_pda
                )
                .await,
                1200000
            );
            assert_eq!(
                utils::get_token_account_balance(
                    &test_setup.program_test_ctx,
                    lm_staking_lm_reward_token_vault_pda
                )
                .await,
                143333333332
            );
        }
    }

    // Claims tokens for alice/martin/paul
    // Should get different share

    let alice_staking_reward_token_account_address =
        utils::find_associated_token_account(&alice.pubkey(), &cortex_stake_reward_mint).0;

    let martin_staking_reward_token_account_address =
        utils::find_associated_token_account(&martin.pubkey(), &cortex_stake_reward_mint).0;

    let paul_staking_reward_token_account_address =
        utils::find_associated_token_account(&paul.pubkey(), &cortex_stake_reward_mint).0;

    // Claim when there is one round worth of rewards to claim
    {
        // Claim alice
        {
            let balance_before = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                alice_staking_reward_token_account_address,
            )
            .await;

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

            let balance_after = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                alice_staking_reward_token_account_address,
            )
            .await;

            assert_eq!(balance_after - balance_before, 150000);
        }

        // Claim martin
        {
            let balance_before = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                martin_staking_reward_token_account_address,
            )
            .await;

            test_instructions::claim_stakes(
                &test_setup.program_test_ctx,
                martin,
                &test_setup.payer_keypair,
                &martin.pubkey(),
                &test_setup.pool_pda,
                &lm_token_mint_pda,
                &ClaimStakesParams {
                    locked_stake_indexes: None,
                },
            )
            .await
            .unwrap();

            let balance_after = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                martin_staking_reward_token_account_address,
            )
            .await;

            assert_eq!(balance_after - balance_before, 562500);
        }

        // Claim paul
        {
            let balance_before = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                paul_staking_reward_token_account_address,
            )
            .await;

            test_instructions::claim_stakes(
                &test_setup.program_test_ctx,
                paul,
                &test_setup.payer_keypair,
                &paul.pubkey(),
                &test_setup.pool_pda,
                &lm_token_mint_pda,
                &ClaimStakesParams {
                    locked_stake_indexes: None,
                },
            )
            .await
            .unwrap();

            let balance_after = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                paul_staking_reward_token_account_address,
            )
            .await;

            assert_eq!(balance_after - balance_before, 487500);
        }
    }

    // Assert all rewards got distributed
    {
        let lm_staking_account =
            utils::get_account::<Staking>(&test_setup.program_test_ctx, lm_staking_pda).await;

        // Accept dust due to precision loss
        assert!(lm_staking_account.resolved_reward_token_amount <= 100);
    }
}
