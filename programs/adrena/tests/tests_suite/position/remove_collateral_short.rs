use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            InitUserProfileParams, OpenPositionShortParams, RemoveCollateralShortParams,
        },
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

// Check position within min_leverage <> max_leverage and remove collateral within min_leverage <> max_leverage
// Check position within min_leverage <> max_leverage and remove collateral > max leverage
// Check position > max leverage and remove collateral > max leverage
// Check position < min leverage and remove collateral < min leverage and remove collateral > min leverage
pub async fn remove_collateral_short() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(10_000_000, USDC_DECIMALS),
                    "eth" => utils::scale(50_000, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(2_000_000, USDC_DECIMALS),
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
                liquidity_amount: utils::scale(2_000_000, USDC_DECIMALS),
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
        utils::scale(10_000_000, Cortex::LM_DECIMALS),
        utils::scale(10_000_000, Cortex::LM_DECIMALS),
        utils::scale(10_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

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

    // Martin: Open 1 ETH short position x1.1001
    let position_pda = test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        usdc_mint,
        OpenPositionShortParams {
            // max price paid (slippage implied)
            price: utils::scale(1_450, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1_000_000, Cortex::USD_DECIMALS),
            leverage: 11_001,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // Remove collateral within min_leverage <> max_leverage borders should succeed
    {
        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(1000, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let martin_usdc_pda = utils::find_associated_token_account(&martin.pubkey(), usdc_mint).0;
        let martin_usdc_balance =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_pda).await;

        assert_eq!(martin_usdc_balance, 1000999000999)
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Remove too much collateral
    {
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(999_000, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }

    // Remove very little collateral
    {
        test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(1, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let martin_usdc_pda = utils::find_associated_token_account(&martin.pubkey(), usdc_mint).0;
        let martin_usdc_balance =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_pda).await;

        assert_eq!(martin_usdc_balance, 1000999999999)
    }

    // Remove exactly all collateral
    {
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: 997898900000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }

    // Remove all collateral - 1 so max leverage exceeded
    {
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(997898800000, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }
    // Remove 0 collateral_usd
    {
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: 0,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }

    // test max leverage, cannot test position > max and add collateral < max because remove collateral increases leverage
    {
        // Make ETH price to increase in order to make the opened position > max leverage
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(2_850, Cortex::PRICE_DECIMALS),
                28500000000, // 10bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position > max leverage and remove collateral > max leverage
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(1, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());

        // Set price to make position < max leverage
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(2_800, Cortex::PRICE_DECIMALS),
                28000000000, // 10bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position is < max leverage but remove collateral > max leverage
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(200_000, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());
    }

    // test min leverage
    {
        // Make ETH price to decrease in order to make the opened position < min leverage
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(1_000, Cortex::PRICE_DECIMALS),
                10000000000, // 10bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position < min leverage and remove collateral < min leverage
        assert!(test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(1, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());

        // Position < min average and remove collateral > min leverage
        test_instructions::remove_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            RemoveCollateralShortParams {
                collateral_usd: utils::scale(400_000, Cortex::USD_DECIMALS),
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
