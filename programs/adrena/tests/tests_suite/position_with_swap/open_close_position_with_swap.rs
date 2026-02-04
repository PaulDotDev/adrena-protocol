use {
    crate::{
        test_instructions,
        utils::{
            self, find_associated_token_account, get_token_account_balance, ChaosLabsFeedIdEnum,
            SetupCustodyOracleParam,
        },
    },
    adrena::{
        instructions::{
            ClosePositionLongParams, GetOpenPositionWithSwapAmountAndFeesParams,
            InitUserProfileParams, OpenPositionWithSwapParams,
        },
        state::{
            cortex::Cortex,
            position::Side,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const ETH_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;
const BTC_DECIMALS: u8 = 6;

pub async fn open_close_position_with_swap() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                    "btc" => utils::scale(50, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                    "btc" => utils::scale(0, BTC_DECIMALS),
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
                    target_ratio: utils::ratio_from_percentage(40.0),
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
                liquidity_amount: utils::scale(15_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(15.0),
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
                liquidity_amount: utils::scale(10, ETH_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "btc",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(15.0),
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
                liquidity_amount: 500000,
                payer_user_name: "alice",
            },
        ],
        utils::scale(200_000, Cortex::LM_DECIMALS),
        utils::scale(300_000, Cortex::LM_DECIMALS),
        utils::scale(500_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let eth_mint = &test_setup.get_mint_by_name("eth");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    let martin_usdc_ata = find_associated_token_account(&martin.pubkey(), usdc_mint).0;
    let martin_eth_ata = find_associated_token_account(&martin.pubkey(), eth_mint).0;
    let martin_btc_ata = find_associated_token_account(&martin.pubkey(), btc_mint).0;

    let mut martin_usdc_balance_before =
        get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    let mut martin_eth_balance_before =
        get_token_account_balance(&test_setup.program_test_ctx, martin_eth_ata).await;
    let mut martin_btc_balance_before =
        get_token_account_balance(&test_setup.program_test_ctx, martin_btc_ata).await;

    // Init required user profile
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

    // Check preshot of what's happening
    {
        let open_position_with_swap_amount_and_fees =
            test_instructions::get_open_position_with_swap_amount_and_fees(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                eth_mint,
                btc_mint,
                btc_mint,
                GetOpenPositionWithSwapAmountAndFeesParams {
                    collateral_amount: 5000000,
                    leverage: 40_000, // x4
                    side: Side::Long.into(),
                    oracle_prices: Some(
                        utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                            .await,
                    ),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            open_position_with_swap_amount_and_fees.entry_price,
            300000000000000
        );
        assert_eq!(
            open_position_with_swap_amount_and_fees.liquidation_price,
            226606354596168
        );
        assert_eq!(open_position_with_swap_amount_and_fees.swap_fee_in, 5000);
        assert_eq!(open_position_with_swap_amount_and_fees.swap_fee_out, 1);
    }

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
            leverage: 40_000, // x4
            // max price paid for BTC when opening the position (slippage implied)
            price: utils::scale(30_400, Cortex::PRICE_DECIMALS),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    let mut martin_usdc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    let mut martin_eth_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_eth_ata).await;
    let mut martin_btc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_btc_ata).await;

    // Double check numbers after swap & position opening
    {
        assert_eq!(martin_usdc_balance_before, martin_usdc_balance_after);
        assert_eq!(
            martin_eth_balance_before - 9_000_000,
            martin_eth_balance_after
        );
        assert_eq!(martin_btc_balance_before, martin_btc_balance_after);
    }

    martin_usdc_balance_before = martin_usdc_balance_after;
    martin_eth_balance_before = martin_eth_balance_after;
    martin_btc_balance_before = martin_btc_balance_after;

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Martin: Increase the ETH position
    test_instructions::open_or_increase_position_with_swap_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        btc_mint,
        OpenPositionWithSwapParams {
            // Amount of ETH to use as collateral
            // $15 of collateral
            collateral: 10000000,
            // $30 position
            leverage: 40_000, // x4
            // max price paid for BTC when opening the position (slippage implied)
            price: utils::scale(30_400, Cortex::PRICE_DECIMALS),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    martin_usdc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    martin_eth_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_eth_ata).await;
    martin_btc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_btc_ata).await;

    // Double check numbers after position increase
    {
        assert_eq!(martin_usdc_balance_before, martin_usdc_balance_after);
        assert_eq!(
            martin_eth_balance_before - 10_000_000,
            martin_eth_balance_after
        );
        assert_eq!(martin_btc_balance_before, martin_btc_balance_after);
    }

    martin_usdc_balance_before = martin_usdc_balance_after;
    martin_eth_balance_before = martin_eth_balance_after;
    martin_btc_balance_before = martin_btc_balance_after;

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
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    martin_usdc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    martin_eth_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_eth_ata).await;
    martin_btc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_btc_ata).await;

    // Double check numbers after position closing
    {
        assert_eq!(martin_usdc_balance_before, martin_usdc_balance_after);
        assert_eq!(martin_eth_balance_before, martin_eth_balance_after);
        assert_eq!(martin_btc_balance_after - martin_btc_balance_before, 930);
    }

    /////

    martin_usdc_balance_before = martin_usdc_balance_after;
    martin_eth_balance_before = martin_eth_balance_after;
    martin_btc_balance_before = martin_btc_balance_after;

    let _ = test_instructions::open_or_increase_position_with_swap_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        usdc_mint,
        btc_mint,
        OpenPositionWithSwapParams {
            // Amount of USDC to use as collateral
            // $10 of collateral
            collateral: utils::scale(10, USDC_DECIMALS),
            // $100 position
            leverage: 100_000, // x10
            price: utils::scale(29_500, Cortex::PRICE_DECIMALS),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    martin_usdc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    martin_eth_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_eth_ata).await;
    martin_btc_balance_after =
        get_token_account_balance(&test_setup.program_test_ctx, martin_btc_ata).await;

    // Double check numbers after position increase
    {
        assert_eq!(
            martin_usdc_balance_before,
            martin_usdc_balance_after + 10_000000
        );
        assert_eq!(martin_eth_balance_before, martin_eth_balance_after);
        assert_eq!(martin_btc_balance_before, martin_btc_balance_after);
    }
}
