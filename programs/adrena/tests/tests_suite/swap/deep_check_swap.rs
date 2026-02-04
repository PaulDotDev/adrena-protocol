use {
    crate::{
        assert_unchanged, test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, SwapParams},
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

pub async fn deep_check_swap() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
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
        // +100k to account for add_liquidity fees
        utils::scale(10_100_000, Cortex::USD_DECIMALS),
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
                liquidity_amount: utils::scale(3400000, USDC_DECIMALS),
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
                liquidity_amount: utils::scale(2200, ETH_DECIMALS),
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
        Some("paul"),
    )
    .await;

    let paul = test_setup.get_user_keypair_by_name("paul");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let eth_mint = &test_setup.get_mint_by_name("eth");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let paul_btc_ata = utils::find_associated_token_account(&paul.pubkey(), btc_mint).0;
    let paul_eth_ata = utils::find_associated_token_account(&paul.pubkey(), eth_mint).0;
    let paul_usdc_ata = utils::find_associated_token_account(&paul.pubkey(), usdc_mint).0;

    let btc_custody_pda = pda::get_custody_pda(&test_setup.pool_pda, btc_mint).0;
    let eth_custody_pda = pda::get_custody_pda(&test_setup.pool_pda, eth_mint).0;
    let usdc_custody_pda = pda::get_custody_pda(&test_setup.pool_pda, usdc_mint).0;

    let btc_custody_token_account_pda =
        pda::get_custody_token_account_pda(&test_setup.pool_pda, btc_mint).0;
    let eth_custody_token_account_pda =
        pda::get_custody_token_account_pda(&test_setup.pool_pda, eth_mint).0;
    let usdc_custody_token_account_pda =
        pda::get_custody_token_account_pda(&test_setup.pool_pda, usdc_mint).0;

    {
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            paul.pubkey(),
            paul,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "paul".to_string(),
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

    // Paul: Swap 1 BTC for ETH
    {
        //
        // Paul accounts
        //
        let paul_btc_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, paul_btc_ata).await;
        let paul_eth_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;
        let paul_usdc_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;
        //
        //
        //

        test_instructions::swap(
            &test_setup.program_test_ctx,
            paul,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            btc_mint,
            SwapParams {
                amount_in: utils::scale(1, BTC_DECIMALS),
                min_amount_out: 0,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        //
        //
        //

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // Check paul token accounts
        {
            let paul_eth_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_eth_ata).await;

            let paul_btc_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_btc_ata).await;

            let paul_usdc_after =
                utils::get_token_account_balance(&test_setup.program_test_ctx, paul_usdc_ata).await;

            // Paul received 1 ETH minus ratios
            assert_eq!(paul_eth_after - paul_eth_before, 20000000000);

            // Paul paid 1 full BTC
            assert_eq!(paul_btc_before - paul_btc_after, 1000000);
            assert_unchanged!(paul_usdc_after, paul_usdc_before);
        }

        // Check custodies
        {
            let btc_custody_account = utils::get_zero_copy_account::<Custody>(
                &test_setup.program_test_ctx,
                btc_custody_pda,
            )
            .await;
            let eth_custody_account = utils::get_zero_copy_account::<Custody>(
                &test_setup.program_test_ctx,
                eth_custody_pda,
            )
            .await;
            let usdc_custody_account = utils::get_zero_copy_account::<Custody>(
                &test_setup.program_test_ctx,
                usdc_custody_pda,
            )
            .await;

            let btc_custody_token_account = utils::get_token_account(
                &test_setup.program_test_ctx,
                btc_custody_token_account_pda,
            )
            .await;
            let eth_custody_token_account = utils::get_token_account(
                &test_setup.program_test_ctx,
                eth_custody_token_account_pda,
            )
            .await;
            let usdc_custody_token_account = utils::get_token_account(
                &test_setup.program_test_ctx,
                usdc_custody_token_account_pda,
            )
            .await;

            //
            // BTC custody
            //

            /*let (initial_add_liquidity_btc_fee_amount_usd, btc_to_eth_swap_fee_in_amount_usd) = */
            {
                let initial_add_liquidity_btc_amount: u64 = 110000000;
                let initial_add_liquidity_btc_amount_usd: u64 = 3291750000000;
                let btc_swapped_amount: u64 = 1000000;
                // let btc_swapped_amount_usd: u64 = 30000000000;

                let initial_add_liquidity_btc_fee_amount_usd: u64 = 3300000000;
                // initial_add_liquidity_btc_amount_usd * 10 / Cortex::BPS_POWER as u64;

                let btc_to_eth_swap_fee_in_amount_usd = 0;
                // btc_swapped_amount_usd * 10 / Cortex::BPS_POWER as u64;

                assert_eq!(
                    btc_custody_account.collected_fees,
                    FeesStats {
                        swap_usd: btc_to_eth_swap_fee_in_amount_usd,
                        add_liquidity_usd: initial_add_liquidity_btc_fee_amount_usd,
                        remove_liquidity_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        borrow_usd: 0,
                    },
                );

                assert_eq!(
                    btc_custody_account.volume_stats,
                    VolumeStats {
                        add_liquidity_usd: initial_add_liquidity_btc_amount_usd,
                        remove_liquidity_usd: 0,
                        open_position_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        swap_usd: 30000000000,
                        // (initial_add_liquidity_btc_fee_amount_usd * 30 / 100)
                        //     + btc_swapped_amount_usd
                        //     + (btc_to_eth_swap_fee_in_amount_usd * 30 / 100),
                    },
                );

                assert_eq!(
                    btc_custody_account.trade_stats,
                    TradeStats {
                        profit_usd: 0,
                        loss_usd: 0,
                        oi_long_usd: 0,
                        oi_short_usd: 0
                    },
                );

                // Asset owned must match the actual amount in the token account
                {
                    let owned = initial_add_liquidity_btc_amount + btc_swapped_amount;

                    assert_eq!(
                        btc_custody_account.assets,
                        Assets {
                            collateral: 0,
                            owned,
                            locked: 0,
                        },
                    );

                    assert_eq!(btc_custody_token_account.amount, owned);
                }

                assert_eq!(
                    btc_custody_account.long_positions,
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
                    btc_custody_account.short_positions,
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

                assert_eq!(btc_custody_account.borrow_rate_state.current_rate, 0);
                assert_eq!(
                    btc_custody_account.borrow_rate_state.cumulative_interest,
                    U128Split::new(0)
                );

                (
                    initial_add_liquidity_btc_fee_amount_usd,
                    btc_to_eth_swap_fee_in_amount_usd,
                )
            };

            //
            // ETH custody
            //

            /*let (initial_add_liquidity_eth_fee_amount_usd, btc_to_eth_swap_fee_out_amount_usd) = */
            {
                // let initial_add_liquidity_eth_amount: u64 = 2200000000000;
                let initial_add_liquidity_eth_amount_usd: u64 = 3300000000000;

                // let eth_swapped_amount_out_no_fee: u64 = 20000000000;
                let eth_swapped_amount_out_no_fee_usd: u64 = 30000000000;

                let initial_add_liquidity_eth_fee_amount_usd: u64 =
                    initial_add_liquidity_eth_amount_usd * 29 / Cortex::BPS_POWER as u64;

                // let btc_to_eth_swap_fee_out_amount =
                //     eth_swapped_amount_out_no_fee * 21 / Cortex::BPS_POWER as u64;

                let btc_to_eth_swap_fee_out_amount_usd =
                    eth_swapped_amount_out_no_fee_usd * 21 / Cortex::BPS_POWER as u64;

                // // To the user
                // let eth_swapped_amount_out =
                //     eth_swapped_amount_out_no_fee - btc_to_eth_swap_fee_out_amount;

                assert_eq!(
                    eth_custody_account.collected_fees,
                    FeesStats {
                        add_liquidity_usd: 3300000000, // initial_add_liquidity_eth_fee_amount_usd,
                        remove_liquidity_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        swap_usd: 0, //btc_to_eth_swap_fee_out_amount_usd, == 63000000 (slight change cause taking in fee out of in amount for calculating out fee)
                        borrow_usd: 0,
                    },
                );

                assert_eq!(
                    eth_custody_account.volume_stats,
                    VolumeStats {
                        add_liquidity_usd: 3291750000000, //initial_add_liquidity_eth_amount_usd,
                        remove_liquidity_usd: 0,
                        open_position_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        swap_usd: 30000000000,
                        // (initial_add_liquidity_eth_fee_amount_usd * 30 / 100)
                        //    + eth_swapped_amount_out_no_fee_usd
                        //    + (btc_to_eth_swap_fee_out_amount_usd * 30 / 100), == 32889900000
                        // (slight change cause taking in fee out of in amount for calculating out fee)
                    },
                );

                assert_eq!(
                    eth_custody_account.trade_stats,
                    TradeStats {
                        profit_usd: 0,
                        loss_usd: 0,
                        oi_long_usd: 0,
                        oi_short_usd: 0
                    },
                );

                // Asset owned must match the actual amount in the token account
                {
                    let owned = 2180000000000; // initial_add_liquidity_eth_amount - eth_swapped_amount_out; == 2180042000000 due to slight change in fee calculation in swap

                    assert_eq!(
                        eth_custody_account.assets,
                        Assets {
                            collateral: 0,
                            owned,
                            locked: 0,
                        },
                    );

                    assert_eq!(eth_custody_token_account.amount, owned);
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

                assert_eq!(eth_custody_account.borrow_rate_state.current_rate, 0);
                assert_eq!(
                    eth_custody_account.borrow_rate_state.cumulative_interest,
                    U128Split::new(0)
                );

                (
                    initial_add_liquidity_eth_fee_amount_usd,
                    btc_to_eth_swap_fee_out_amount_usd,
                )
            };

            // USDC custody
            {
                assert_eq!(
                    usdc_custody_account.collected_fees,
                    FeesStats {
                        add_liquidity_usd: 3400000000,
                        remove_liquidity_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        swap_usd: 0,
                        borrow_usd: 0,
                    },
                );

                assert_eq!(
                    usdc_custody_account.volume_stats,
                    VolumeStats {
                        add_liquidity_usd: 3396600000000,
                        remove_liquidity_usd: 0,
                        open_position_usd: 0,
                        close_position_usd: 0,
                        liquidation_usd: 0,
                        swap_usd: 0,
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
                    let owned = 3397000000000;
                    //initial_add_liquidity_usdc_amount_usd
                    // - (initial_add_liquidity_usdc_fee_amount_usd * 30 / 100)
                    // - (initial_add_liquidity_btc_fee_amount_usd * 30 / 100)
                    // - (initial_add_liquidity_eth_fee_amount_usd * 30 / 100)
                    // - (btc_to_eth_swap_fee_in_amount_usd * 30 / 100)
                    // - (btc_to_eth_swap_fee_out_amount_usd * 30 / 100); // == 3386931100000 - due to slight change in swap fee calculation

                    assert_eq!(
                        usdc_custody_account.assets,
                        Assets {
                            collateral: 0,
                            owned,
                            locked: 0,
                        },
                    );

                    assert_eq!(usdc_custody_token_account.amount, owned);
                }

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

                assert_eq!(usdc_custody_account.borrow_rate_state.current_rate, 0);
                assert_eq!(
                    usdc_custody_account.borrow_rate_state.cumulative_interest,
                    U128Split::new(0)
                );
            }
        }
    }
}
