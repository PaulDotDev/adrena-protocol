use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddCollateralLongParams, InitUserProfileParams, OpenPositionLongParams},
        state::{
            cortex::Cortex,
            custody::PricingParams,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const ETH_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

// Check position within min_leverage <> max_leverage and add collateral within min_leverage <> max_leverage
// Check position within min_leverage <> max_leverage and add collateral < min leverage
// Check position > max leverage and add collateral > max leverage and add collateral < max leverage
// Check position < min leverage and add collateral < min leverage
pub async fn add_collateral_long() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1_000_000, USDC_DECIMALS),
                    "eth" => utils::scale(10_000, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(10_000, USDC_DECIMALS),
                    "eth" => utils::scale(2000, ETH_DECIMALS),
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
        utils::scale(20_000_000, Cortex::USD_DECIMALS),
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
                liquidity_amount: utils::scale(1_000_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(100.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1500, Cortex::PRICE_DECIMALS),
                        initial_conf: 15000000000, // 10 bps
                        oracle_name: LimitedString::new("eth"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::ETH,
                    },
                    trade_oracle: None,
                    pricing_params: Some(PricingParams {
                        ..utils::fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10_000, ETH_DECIMALS),
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

    let martin = test_setup.get_user_keypair_by_name("martin");

    let eth_mint = &test_setup.get_mint_by_name("eth");

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

    // Martin: Open 1 ETH long position x5
    test_instructions::open_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        OpenPositionLongParams {
            // max price paid (slippage implied)
            price: utils::scale(1_550, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1, ETH_DECIMALS),
            leverage: 50_000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // position within min_leverage <> max_leverage and add collateral within min_leverage <> max_leverage borders
    {
        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(1, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let martin_eth_pda = utils::find_associated_token_account(&martin.pubkey(), eth_mint).0;

        let martin_eth_balance =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_eth_pda).await;

        assert_eq!(martin_eth_balance, utils::scale(1998, ETH_DECIMALS))
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Position within min_leverage <> max_leverage and add collateral within min_leverage <> max_leverage
    {
        test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(2, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let martin_eth_pda_after =
            utils::find_associated_token_account(&martin.pubkey(), eth_mint).0;

        let martin_eth_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_eth_pda_after)
                .await;

        assert_eq!(martin_eth_balance_after, utils::scale(1996, ETH_DECIMALS))
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Position leverage  within min_leverage <> max_leverage and add collateral < min leverage
    {
        assert!(test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(2, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }

    // check max leverage
    {
        // Make ETH price to drop in order to make the opened position leverage go > max leverage
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(250, Cortex::PRICE_DECIMALS),
                2500000000, // 10 bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position > max leverage and add collateral > max leverage
        assert!(test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(1, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());

        // position > max leverage and add collateral < max leverage
        test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(10, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();
    }

    // check min leverage, cannot test position < min and add collateral > min because add collateral reduces leverage
    {
        // Make ETH price to increase in order to make the opened position leverage go < min leverage
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(2_550, Cortex::PRICE_DECIMALS),
                25500000000, // 10 bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position < min leverage and add collateral < min leverage
        assert!(test_instructions::add_collateral_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddCollateralLongParams {
                collateral: utils::scale(1, ETH_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }
}
