use {
    crate::{
        assert_unchanged, test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionLongParams},
        state::{
            cortex::Cortex,
            custody::{
                Assets, Custody, FeesStats, PositionsAccounting, StableLockedAmountStat,
                TradeStats, VolumeStats,
            },
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::{limited_string::LimitedString, u128_split::U128Split},
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;
const BTC_DECIMALS: u8 = 6;

pub async fn deep_check_liquidate_position_long() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "eth" => utils::scale(1, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(4_000_000, USDC_DECIMALS),
                    "eth"  => utils::scale(3_000, ETH_DECIMALS),
                    "btc"  => utils::scale(200, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "executioner",
                token_balances: hashmap! {},
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
                    min_ratio: utils::ratio_from_percentage(31.0),
                    max_ratio: utils::ratio_from_percentage(37.0),
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
                liquidity_amount: utils::scale(3_400_000, USDC_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
                    min_ratio: utils::ratio_from_percentage(30.0),
                    max_ratio: utils::ratio_from_percentage(36.0),
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
                liquidity_amount: utils::scale(2_200, ETH_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "btc",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
                    min_ratio: utils::ratio_from_percentage(30.0),
                    max_ratio: utils::ratio_from_percentage(36.0),
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
                liquidity_amount: utils::scale(110, BTC_DECIMALS),
                payer_user_name: "martin",
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
    let executioner = test_setup.get_user_keypair_by_name("executioner");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let eth_mint = &test_setup.get_mint_by_name("eth");

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Init user profiles
    {
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            alice.pubkey(),
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "alice".to_string(),
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

        {
            let cortex_pda = utils::pda::get_cortex_pda().0;
            let cortex_account =
                utils::get_zero_copy_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda)
                    .await;

            let a = cortex_account.user_profiles_count;
            assert_eq!(a, 1);
        }
    }

    {
        // GIVEN --------------------------------------------------------------
        let eth_max_open_price_usdc = utils::scale(1_550, Cortex::PRICE_DECIMALS);
        let collateral_eth = utils::scale(1, ETH_DECIMALS);
        let collateral_usd_value = 1496250000;
        let leverage = 40;

        let usdc_custody_pda = pda::get_custody_pda(&test_setup.pool_pda, usdc_mint).0;
        let eth_custody_pda = pda::get_custody_pda(&test_setup.pool_pda, eth_mint).0;
        let alice_eth_ata = utils::find_associated_token_account(&alice.pubkey(), eth_mint).0;
        let alice_usdc_ata = utils::find_associated_token_account(&alice.pubkey(), usdc_mint).0;

        //
        // Alice accounts
        //
        let alice_eth_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_eth_ata).await;
        let alice_usdc_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

        // WHEN ---------------------------------------------------------------
        // Alice: Open 1 ETH Long position leverage x40 @ 1500
        let position_pda = test_instructions::open_position_long(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            OpenPositionLongParams {
                // max price paid (slippage implied)
                price: eth_max_open_price_usdc,
                collateral: collateral_eth,
                leverage: (leverage * Cortex::BPS_POWER as u64) as u32,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        // Makes ETH price drop to liquidation level
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(1_465, Cortex::PRICE_DECIMALS),
                14800000000, // 10bps
            )
            .await;

        // Executioner: Liquidate Alice ETH position
        test_instructions::liquidate_long(
            &test_setup.program_test_ctx,
            executioner,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            &position_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // THEN ---------------------------------------------------------------
        let eth_custody_after =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, eth_custody_pda)
                .await;
        let usdc_custody_after =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;
        let eth_custody_token_account_after = utils::get_token_account(
            &test_setup.program_test_ctx,
            eth_custody_after.token_account,
        )
        .await;
        let usdc_custody_token_account_after = utils::get_token_account(
            &test_setup.program_test_ctx,
            usdc_custody_after.token_account,
        )
        .await;

        let eth_custody_account =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, eth_custody_pda)
                .await;
        let usdc_custody_account =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;

        let real_position_size = (collateral_eth) * leverage;
        let real_position_size_usd = (collateral_usd_value) * leverage;
        let liquidation_fee = real_position_size * 30 / Cortex::BPS_POWER as u64; // 30 bps fees
        let liquidation_fee_usd = 96000000; //real_position_size_usd * 30 / Cortex::BPS_POWER as u64; // 30 bps fees
        let loss_eth;

        // Check Alice token accounts
        {
            let alice_eth_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, alice_eth_ata).await;

            let alice_usdc_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata)
                    .await;

            // Alice paid close fees in ETH
            assert!(alice_eth_before - alice_eth_after > liquidation_fee); // + loss due to price change
            assert_unchanged!(alice_usdc_before, alice_usdc_after);

            loss_eth = alice_eth_before - alice_eth_after;
        }

        // Check custodies
        {
            // Values taken prior to running the open position --------------------------------
            let initial_add_liquidity_eth_amount = utils::scale(2_200, ETH_DECIMALS);
            let initial_add_liquidity_eth_amount_usd = 3291750000000;
            // let initial_eth_swap_usd_volume = 2_871_000_000;
            let initial_add_liquidity_usdc_amount_usd = 3396600000000;
            let initial_add_liquidity_eth_fee_amount_usd: u64 = 3300000000;
            let initial_add_liquidity_usdc_fee_amount_usd: u64 = 3400000000;
            // let liquidate_position_fee_swap_to_usdc_volume = liquidation_fee_usd * 30 / 100;
            // --------------------------------------------------------------------------------

            assert_eq!(
                eth_custody_account.volume_stats,
                VolumeStats {
                    add_liquidity_usd: initial_add_liquidity_eth_amount_usd,
                    remove_liquidity_usd: 0,
                    open_position_usd: real_position_size_usd,
                    close_position_usd: 0,
                    liquidation_usd: real_position_size_usd,
                    swap_usd: 0,
                },
            );
            assert_eq!(
                usdc_custody_account.volume_stats,
                VolumeStats {
                    add_liquidity_usd: initial_add_liquidity_usdc_amount_usd,
                    remove_liquidity_usd: 0,
                    open_position_usd: 0,
                    close_position_usd: 0,
                    liquidation_usd: 0,
                    swap_usd: 0,
                },
            );

            assert_eq!(
                eth_custody_account.collected_fees,
                FeesStats {
                    add_liquidity_usd: initial_add_liquidity_eth_fee_amount_usd,
                    remove_liquidity_usd: 0,
                    close_position_usd: 0,
                    liquidation_usd: liquidation_fee_usd,
                    swap_usd: 0,
                    borrow_usd: 0,
                },
            );
            assert_eq!(
                usdc_custody_account.collected_fees,
                FeesStats {
                    add_liquidity_usd: initial_add_liquidity_usdc_fee_amount_usd,
                    remove_liquidity_usd: 0,
                    close_position_usd: 0,
                    liquidation_usd: 0,
                    swap_usd: 0,
                    borrow_usd: 0,
                },
            );

            assert_eq!(
                eth_custody_account.trade_stats,
                TradeStats {
                    profit_usd: 0,
                    loss_usd: 1492500000,
                    oi_long_usd: 0,
                    oi_short_usd: 0,
                },
            );
            assert_eq!(
                usdc_custody_account.trade_stats,
                TradeStats {
                    profit_usd: 0,
                    loss_usd: 0,
                    oi_long_usd: 0,
                    oi_short_usd: 0
                },
            );

            // Asset owned must match the actual amount in the token account
            {
                let owned = initial_add_liquidity_eth_amount
                    + liquidation_fee
                    + (loss_eth - liquidation_fee);

                assert_eq!(
                    eth_custody_account.assets,
                    Assets {
                        collateral: 0,
                        owned,
                        locked: 0,
                    },
                );

                // All that's been deposited
                assert_eq!(eth_custody_token_account_after.amount, owned);
            }

            // Asset owned must match the actual amount in the token account
            {
                let owned = 3396971200000;
                // initial_add_liquidity_usdc_amount
                //     - (initial_add_liquidity_usdc_fee_amount_usd * 30 / 100)
                //     - (initial_add_liquidity_btc_fee_amount_usd * 30 / 100)
                //     - (initial_add_liquidity_eth_fee_amount_usd * 30 / 100)
                //     - (liquidation_fee_usd * 30 / 100);

                assert_eq!(
                    usdc_custody_account.assets,
                    Assets {
                        collateral: 0,
                        owned,
                        locked: 0,
                    },
                );

                assert_eq!(usdc_custody_token_account_after.amount, owned);
            }

            assert_eq!(
                eth_custody_account.long_positions,
                PositionsAccounting {
                    open_positions: 0,
                    size_usd: 0,
                    borrow_size_usd: 0,
                    locked_amount: 0,
                    weighted_price: U128Split::new(0),
                    total_quantity: U128Split::new(0),
                    cumulative_interest_usd: 0,
                    collateral_usd: 0,
                    cumulative_interest_snapshot: U128Split::new(0),
                    exit_fee_usd: 0,
                    stable_locked_amount: [
                        StableLockedAmountStat::default(),
                        StableLockedAmountStat::default(),
                    ],
                },
            );
            assert_eq!(
                usdc_custody_account.long_positions,
                PositionsAccounting {
                    open_positions: 0,
                    size_usd: 0,
                    borrow_size_usd: 0,
                    locked_amount: 0,
                    weighted_price: U128Split::new(0),
                    total_quantity: U128Split::new(0),
                    cumulative_interest_usd: 0,
                    collateral_usd: 0,
                    cumulative_interest_snapshot: U128Split::new(0),
                    exit_fee_usd: 0,
                    stable_locked_amount: [
                        StableLockedAmountStat::default(),
                        StableLockedAmountStat::default(),
                    ],
                },
            );

            assert_eq!(
                eth_custody_account.short_positions,
                PositionsAccounting {
                    open_positions: 0,
                    size_usd: 0,
                    borrow_size_usd: 0,
                    locked_amount: 0,
                    weighted_price: U128Split::new(0),
                    total_quantity: U128Split::new(0),
                    cumulative_interest_usd: 0,
                    collateral_usd: 0,
                    cumulative_interest_snapshot: U128Split::new(0),
                    exit_fee_usd: 0,
                    stable_locked_amount: [
                        StableLockedAmountStat::default(),
                        StableLockedAmountStat::default(),
                    ],
                },
            );
            assert_eq!(
                usdc_custody_account.short_positions,
                PositionsAccounting {
                    open_positions: 0,
                    size_usd: 0,
                    borrow_size_usd: 0,
                    locked_amount: 0,
                    weighted_price: U128Split::new(0),
                    total_quantity: U128Split::new(0),
                    cumulative_interest_usd: 0,
                    collateral_usd: 0,
                    cumulative_interest_snapshot: U128Split::new(0),
                    exit_fee_usd: 0,
                    stable_locked_amount: [
                        StableLockedAmountStat::default(),
                        StableLockedAmountStat::default(),
                    ],
                },
            );

            assert_eq!(eth_custody_account.borrow_rate_state.current_rate, 0);
            assert_eq!(usdc_custody_account.borrow_rate_state.current_rate, 0);

            assert_eq!(
                eth_custody_account.borrow_rate_state.cumulative_interest,
                U128Split::new(0)
            );
            assert_eq!(
                usdc_custody_account.borrow_rate_state.cumulative_interest,
                U128Split::new(0)
            );
        }
    }
}
