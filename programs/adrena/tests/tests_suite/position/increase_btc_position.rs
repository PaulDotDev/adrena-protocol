use {
    crate::{
        test_instructions::{self},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionWithSwapParams},
        state::{
            cortex::Cortex,
            position::Position,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 6;
const BTC_DECIMALS: u8 = 8;

#[allow(deprecated)]
pub async fn increase_btc_position() {
    let test_setup: utils::TestSetup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(50000000, USDC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(200000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(150000000, USDC_DECIMALS),
                    "eth"  => utils::scale(1000000, ETH_DECIMALS),
                    "btc"  => utils::scale(100000, BTC_DECIMALS),
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
        utils::scale(100000000, Cortex::USD_DECIMALS),
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
                        initial_conf: 10000000, // 10 bps
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(15000000, USDC_DECIMALS),
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
                liquidity_amount: utils::scale(10000, ETH_DECIMALS),
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
                liquidity_amount: utils::scale(1000, BTC_DECIMALS),
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

    let martin = test_setup.get_user_keypair_by_name("martin");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    {
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            martin.pubkey(),
            martin,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "martin".to_string(),
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

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 300000000000000);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(29000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        {
            let pnl = test_instructions::get_pnl(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                &position_pda,
                Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            )
            .await
            .unwrap();

            assert_eq!(pnl.profit_usd, 634000);
            assert_eq!(pnl.loss_usd, 0);
            assert_eq!(pnl.exit_fee_usd, 32000);
        }

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        {
            let pnl = test_instructions::get_pnl(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                &position_pda,
                Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            )
            .await
            .unwrap();

            assert_eq!(pnl.profit_usd, 601999);
            assert_eq!(pnl.loss_usd, 0);
            assert_eq!(pnl.exit_fee_usd, 64000);
        }

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 294915254237288);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 293258426966292);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 292436974789915);
    }

    test_setup
        .update_oracle_price(
            "btc",
            utils::scale(30000, Cortex::PRICE_DECIMALS),
            300000000000, // 10 bps
        )
        .await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 100000,
                leverage: 20000, // x2
                price: utils::scale(30000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 300000000000000);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(29000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 292500000000000);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 291429491307147);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 291000901713255);
    }

    test_setup
        .update_oracle_price(
            "btc",
            utils::scale(30000, Cortex::PRICE_DECIMALS),
            300000000000, // 10 bps
        )
        .await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 299969092345332);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(29000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 299037952627697);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 298265895966509);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 297615362429414);
    }

    test_setup
        .update_oracle_price(
            "btc",
            utils::scale(30000, Cortex::PRICE_DECIMALS),
            300000000000, // 10 bps
        )
        .await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 10000000,
                leverage: 20000, // x2
                price: utils::scale(30000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 299182570245871);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(29000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 298938765792750);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 298707911031281);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: 300000,
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 298488680288620);
    }

    test_setup
        .update_oracle_price(
            "btc",
            utils::scale(30000, Cortex::PRICE_DECIMALS),
            300000000000, // 10 bps
        )
        .await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 298645689012015);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(20000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(9900, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 292641564887499);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(9900, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 287326395770552);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(9900, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 282588032792129);
    }

    test_setup
        .update_oracle_price(
            "btc",
            utils::scale(30000, Cortex::PRICE_DECIMALS),
            300000000000, // 10 bps
        )
        .await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(100, BTC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(30000, Cortex::PRICE_DECIMALS),
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

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 299998203938874);

        test_setup
            .update_oracle_price(
                "btc",
                utils::scale(29000, Cortex::PRICE_DECIMALS),
                290000000000, // 10 bps
            )
            .await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(30, BTC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 297693035902700);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(30, BTC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 296253422756788);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            btc_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(30, BTC_DECIMALS),
                leverage: 20000, // x2
                price: utils::scale(29000, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 295267672668320);
    }
}
