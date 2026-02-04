use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidStakeParams, AddLockedStakeParams, AddVestParams, BucketName,
            ClaimStakesParams, ClosePositionLongParams, IncreasePositionLongParams,
            InitUserProfileParams, OpenPositionLongParams, OpenPositionWithSwapParams,
            RemoveLiquidStakeParams, RemoveLiquidityParams, SwapParams,
        },
        state::{
            cortex::Cortex,
            pool::Pool,
            staking::StakingRound,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;
const BTC_DECIMALS: u8 = 6;

pub async fn basic_interactions() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(5000000, USDC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(100000, USDC_DECIMALS),
                    "eth"  => utils::scale(2000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(150000, USDC_DECIMALS),
                    "eth"  => utils::scale(1000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
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
            utils::MintParam {
                name: "btc",
                decimals: BTC_DECIMALS,
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
                    target_ratio: utils::ratio_from_percentage(34.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1, Cortex::PRICE_DECIMALS),
                        initial_conf: 10000000,
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(1500000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
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
                liquidity_amount: utils::scale(1000, ETH_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "btc",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(30000, Cortex::PRICE_DECIMALS),
                        initial_conf: 300000000000, // 10 bps
                        oracle_name: LimitedString::new("btc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::BTC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: 50000000,
                payer_user_name: "martin",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        Some("paul"),
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");
    let paul = test_setup.get_user_keypair_by_name("paul");

    let cortex_stake_reward_mint = test_setup.get_fee_redistribution_mint();

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let eth_mint = &test_setup.get_mint_by_name("eth");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Change the protocol fee recipient
    {
        let cortex_pda = pda::get_cortex_pda().0;
        let mut cortex_account =
            utils::get_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        let old_one = cortex_account.protocol_fee_recipient;

        let alice_ata = utils::find_associated_token_account(
            &alice.pubkey(),
            &cortex_account.fee_redistribution_mint,
        )
        .0;

        test_instructions::set_protocol_fee_recipient(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            &alice_ata,
            &cortex_stake_reward_mint,
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        cortex_account =
            utils::get_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        assert_eq!(cortex_account.protocol_fee_recipient, alice_ata);

        test_instructions::set_protocol_fee_recipient(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            &old_one,
            &cortex_stake_reward_mint,
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        cortex_account =
            utils::get_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        assert_eq!(cortex_account.protocol_fee_recipient, old_one);
    }

    // Init missing user profiles
    {
        {
            let names = ["martin", "alice", "paul"];
            for (i, user) in [martin, alice, paul].iter().enumerate() {
                test_instructions::init_user_profile(
                    &test_setup.program_test_ctx,
                    user.pubkey(),
                    user,
                    &test_setup.payer_keypair,
                    InitUserProfileParams {
                        nickname: names[i].to_string(),
                        profile_picture: ProfilePicture::Zero as u8,
                        wallpaper: Wallpaper::Zero as u8,
                        title: Title::Zero as u8,
                        team: Team::Default as u8,
                        continent: Continent::Default as u8,
                    },
                    None,
                )
                .await
                .unwrap();
            }
        }

        {
            let cortex_pda = utils::pda::get_cortex_pda().0;
            let cortex_account =
                utils::get_zero_copy_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda)
                    .await;

            let a = cortex_account.user_profiles_count;
            assert_eq!(a, 3);
        }
    }

    // Simple open/increase/close position
    {
        // Martin: Open 0.1 ETH position
        let position_pda = test_instructions::open_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            OpenPositionLongParams {
                // max price paid (slippage implied)
                price: utils::scale(1_550, Cortex::PRICE_DECIMALS),
                collateral: 100000000,
                leverage: 11000, // x1.1
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // Martin: Increase 0.1 ETH position
        test_instructions::increase_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            IncreasePositionLongParams {
                // max price paid (slippage implied)
                price: utils::scale(1_550, Cortex::PRICE_DECIMALS),
                collateral: 100000000,
                leverage: 11000, // x1.1
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // Attempt to close before the min position duration - should fail
        assert!(test_instructions::close_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            ClosePositionLongParams {
                // lowest exit price paid (slippage implied)
                price: Some(utils::scale(1_450, Cortex::PRICE_DECIMALS)),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx,)
                        .await,
                ),
                percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
            },
        )
        .await
        .is_err());

        utils::warp_forward(
            &test_setup.program_test_ctx,
            adrena::state::position::MIN_POSITION_OPEN_TIME_SECONDS as i64,
        )
        .await;

        // Martin: Close the ETH position
        test_instructions::close_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            ClosePositionLongParams {
                // lowest exit price paid (slippage implied)
                price: Some(utils::scale(1_451, Cortex::PRICE_DECIMALS)),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
                percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    // Open/close long position with swap
    //
    // Long on BTC using ETH as collateral (auto-swapped for BTC)
    {
        // Attempt to open a position with 7.5$ collateral (too low, min required is 9$) - should fail
        assert!(test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                // Amount of ETH to use as collateral
                // $7.5 of collateral
                collateral: 5000000,
                // $30 position
                leverage: 40000, // x4
                // max price paid for BTC when opening the position (slippage implied)
                price: utils::scale(30_400, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx,)
                        .await,
                ),
            },
        )
        .await
        .is_err());

        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                // Amount of ETH to use as collateral
                // ~$10 of collateral
                collateral: 9000000,
                // $30 position
                leverage: 40000, // x4
                // max price paid for BTC when opening the position (slippage implied)
                price: utils::scale(30_400, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(
            &test_setup.program_test_ctx,
            adrena::state::position::MIN_POSITION_OPEN_TIME_SECONDS as i64,
        )
        .await;

        // Martin: Close the ETH position
        test_instructions::close_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            ClosePositionLongParams {
                // lowest exit price paid (slippage implied)
                price: Some(utils::scale(29_500, Cortex::PRICE_DECIMALS)),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
                percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    // Simple swaps
    {
        let paul_eth_ata = utils::find_associated_token_account(&paul.pubkey(), eth_mint).0;
        let paul_usdc_ata = utils::find_associated_token_account(&paul.pubkey(), usdc_mint).0;

        // Paul: Swap 150 USDC for ETH
        {
            let eth_balance_before =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;

            let usdc_balance_before =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;

            test_instructions::swap(
                &test_setup.program_test_ctx,
                paul,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                eth_mint,
                // The program receives USDC
                usdc_mint,
                SwapParams {
                    amount_in: utils::scale(150, USDC_DECIMALS),
                    min_amount_out: 90000000,
                    oracle_prices: Some(
                        utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                            .await,
                    ),
                },
            )
            .await
            .unwrap();

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            let eth_balance_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;

            let usdc_balance_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;

            // Paid USDC
            assert_eq!(usdc_balance_before - usdc_balance_after, 150000000);

            // Receives ETH
            assert_eq!(eth_balance_after - eth_balance_before, 99999999);
        }

        // Paul: Swap 0.1 ETH for 150 USDC
        {
            let eth_balance_before =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;

            let usdc_balance_before =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;

            test_instructions::swap(
                &test_setup.program_test_ctx,
                paul,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                usdc_mint,
                // The program receives ETH
                eth_mint,
                SwapParams {
                    amount_in: 100000000,
                    min_amount_out: utils::scale(140, USDC_DECIMALS),
                    oracle_prices: Some(
                        utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                            .await,
                    ),
                },
            )
            .await
            .unwrap();

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            let eth_balance_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;

            let usdc_balance_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;

            assert_eq!(eth_balance_before - eth_balance_after, 100000000);
            assert_eq!(usdc_balance_after - usdc_balance_before, 150000000);
        }
    }

    // Remove liquidity
    {
        let alice_lp_token =
            utils::find_associated_token_account(&alice.pubkey(), &test_setup.lp_token_mint_pda).0;

        let alice_lp_token_balance =
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_lp_token).await;

        // Alice: Remove 80% of provided liquidity (1.5k USDC less fees)
        test_instructions::remove_liquidity(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            RemoveLiquidityParams {
                lp_amount_in: alice_lp_token_balance * 80 / 100,
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

    test_instructions::init_user_staking(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &lm_token_mint_pda,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    // Simple vest and claim
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        // Alice: vest 2 token, unlock period from now to in 7 days
        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            alice,
            &AddVestParams {
                amount: utils::scale(2, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        // warp to have tokens to claim
        utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

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

    // UserStaking
    {
        // 0d ADX should be rejected
        assert!(test_instructions::add_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLockedStakeParams {
                amount: utils::scale(1, Cortex::LM_DECIMALS),
                locked_days: 0,
            },
            &lm_token_mint_pda,
            &test_setup.governance_realm_pda,
        )
        .await
        .is_err());

        // Alice: add liquid stake
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

        // Alice: claim stake (nothing to be claimed yet)
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

        // Alice: remove liquid staking
        test_instructions::remove_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            RemoveLiquidStakeParams {
                amount: utils::scale(1, Cortex::LM_DECIMALS),
            },
            &cortex_stake_reward_mint,
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();

        // warps to the next round
        utils::warp_forward(
            &test_setup.program_test_ctx,
            StakingRound::ROUND_MIN_DURATION_SECONDS,
        )
        .await;

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Delete user profiles
    {
        test_instructions::delete_user_profile(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &alice.pubkey(),
            &test_setup.payer_keypair,
        )
        .await
        .unwrap();

        test_instructions::delete_user_profile(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &martin.pubkey(),
            &test_setup.payer_keypair,
        )
        .await
        .unwrap();

        test_instructions::delete_user_profile(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &paul.pubkey(),
            &test_setup.payer_keypair,
        )
        .await
        .unwrap();
    }

    {
        let pool_info_snapshot = test_instructions::get_pool_info_snapshot(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        assert_eq!(pool_info_snapshot.aum_usd, 3290795308731);
        assert_eq!(pool_info_snapshot.lp_token_price, 10020064781);
        assert_eq!(pool_info_snapshot.custodies_info_snapshot.len(), 3);
    }

    let pool = utils::get_account::<Pool>(&test_setup.program_test_ctx, test_setup.pool_pda).await;

    assert_eq!(pool.fees_debt_usd, 0);
    assert_eq!(pool.referrers_fee_debt_usd, 0);
}
