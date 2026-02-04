use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddCollateralShortParams, InitUserProfileParams, OpenPositionShortParams},
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

const JITOSOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

// Check position within min_leverage <> max_leverage and add collateral within min_leverage <> max_leverage
// Check position within min_leverage <> max_leverage and add collateral < min leverage
// Check position > max leverage and add collateral > max leverage and add collateral < max leverage
// Check position < min leverage and add collateral < min leverage
pub async fn add_collateral_short() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1_000_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(10_000, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1_000_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(2000, JITOSOL_DECIMALS),
                },
            },
        ],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
            },
            utils::MintParam {
                name: "jitoSOL",
                decimals: JITOSOL_DECIMALS,
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
                    pricing_params: Some(PricingParams {
                        max_position_locked_usd: utils::scale(8_000_000, Cortex::USD_DECIMALS),
                        ..utils::fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(1_000_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "jitoSOL",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(200, Cortex::PRICE_DECIMALS),
                        initial_conf: 2000000000, // 10 bps
                        oracle_name: LimitedString::new("jitoSOL"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::JITOSOL,
                    },
                    trade_oracle: Some(SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 1000000000, // 10 bps
                        oracle_name: LimitedString::new("sol"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::SOL,
                    }),
                    pricing_params: Some(PricingParams {
                        // Expressed in BPS, with BPS = 10_000
                        // 50_000 = x5, 100_000 = x10
                        max_leverage: 100_000,
                        max_initial_leverage: 100_000,
                        ..utils::fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10_000, JITOSOL_DECIMALS),
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

    let jitosol_mint = &test_setup.get_mint_by_name("jitoSOL");
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

    // Martin: Open long position x1.5
    let position_pda = test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(500, USDC_DECIMALS),
            leverage: 15000,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // position leverage within min_leverage <> max_leverage and add collateral within min_leverage <> max_leverage
    {
        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::add_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            AddCollateralShortParams {
                collateral: utils::scale(100, Cortex::USD_DECIMALS),
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

        assert_eq!(
            martin_usdc_balance,
            utils::scale(999400, Cortex::USD_DECIMALS)
        )
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // position leverage within min_leverage <> max_leverage and add collateral below min_leverage
    {
        assert!(test_instructions::add_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            AddCollateralShortParams {
                collateral: utils::scale(880, Cortex::USD_DECIMALS),
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
        // Make SOL price to increase in order to make the opened position leverage go > max leverage
        test_setup
            .update_oracle_price(
                "sol",
                utils::scale(300, Cortex::PRICE_DECIMALS),
                3000000000, // 10 bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position > max leverage and add collateral > max leverage
        assert!(test_instructions::add_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            AddCollateralShortParams {
                collateral: utils::scale(1, Cortex::USD_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .is_err());

        // position > max leverage and add collateral < max leverage
        test_instructions::add_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            AddCollateralShortParams {
                collateral: utils::scale(1000, Cortex::USD_DECIMALS),
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
        // Make price to drop in order to make the opened position leverage go < min leverage
        test_setup
            .update_oracle_price(
                "sol",
                utils::scale(80, Cortex::PRICE_DECIMALS),
                800000000, // 10 bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // position < min leverage and add collateral < min leverage
        assert!(test_instructions::add_collateral_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            AddCollateralShortParams {
                collateral: utils::scale(1, Cortex::USD_DECIMALS),
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
