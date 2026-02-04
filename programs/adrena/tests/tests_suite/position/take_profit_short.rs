use {
    crate::{
        test_instructions::{self},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionShortParams, SetTakeProfitShortParams},
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

pub async fn take_profit_short() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100, JITOSOL_DECIMALS),
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
                name: "jitoSOL",
                decimals: JITOSOL_DECIMALS,
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
                liquidity_amount: utils::scale(150_000, USDC_DECIMALS),
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
                        initial_conf: 0,
                        oracle_name: LimitedString::new("jitoSOL"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::JITOSOL,
                    },
                    trade_oracle: Some(SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
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
                liquidity_amount: utils::scale(100, JITOSOL_DECIMALS),
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

    // Martin: Open 10 SOL short position x5
    let position_pda = test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        usdc_mint,
        OpenPositionShortParams {
            // max price paid (slippage implied)
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1000, USDC_DECIMALS),
            leverage: 50_000,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // Cancel a non existing take profit (works if set or not set, checks are done in the test_instruction)
    test_instructions::cancel_take_profit(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
    )
    .await
    .unwrap();

    // Martin: set TP on the position
    test_instructions::set_take_profit_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        SetTakeProfitShortParams {
            take_profit_limit_price: utils::scale(80, Cortex::PRICE_DECIMALS),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 10).await;

    // Take Profit fails when limit is not reached
    {
        test_setup
            .update_oracle_price("sol", utils::scale(81, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // Trigger TP automation
        assert!(!utils::execute_take_profit_short_automation(
            &test_setup.program_test_ctx,
            &position_pda,
            &test_setup.payer_keypair,
        )
        .await
        .unwrap());
    }

    // Test the update - Take Profit fails when limit is not reached
    {
        // Martin: set TP on the position
        test_instructions::set_take_profit_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            SetTakeProfitShortParams {
                take_profit_limit_price: utils::scale(78, Cortex::PRICE_DECIMALS),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_setup
            .update_oracle_price("sol", utils::scale(79, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        // Trigger TP automation - should fail
        assert!(!utils::execute_take_profit_short_automation(
            &test_setup.program_test_ctx,
            &position_pda,
            &test_setup.payer_keypair,
        )
        .await
        .unwrap());

        // reset price
        test_setup
            .update_oracle_price("sol", utils::scale(81, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 10).await;

        // Martin: reset TP on the position
        test_instructions::set_take_profit_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            SetTakeProfitShortParams {
                take_profit_limit_price: utils::scale(80, Cortex::PRICE_DECIMALS),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    // Trigger Take Profit
    {
        test_setup
            .update_oracle_price("sol", utils::scale(79, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 10).await;

        // Trigger TP automation
        assert!(utils::execute_take_profit_short_automation(
            &test_setup.program_test_ctx,
            &position_pda,
            &test_setup.payer_keypair,
        )
        .await
        .unwrap());
    }
}
