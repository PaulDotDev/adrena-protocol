use {
    crate::{
        test_instructions::{self},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionShortParams, SetStopLossShortParams},
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

pub async fn stop_loss_short() {
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
            leverage: 50000,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // Cancel a non existing stop loss (works if set or not set, checks are done in the test_instruction)
    test_instructions::cancel_stop_loss(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Martin: set SL on the position
    test_instructions::set_stop_loss_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        SetStopLossShortParams {
            stop_loss_limit_price: utils::scale(120, Cortex::PRICE_DECIMALS),
            close_position_price: Some(utils::scale(140, Cortex::PRICE_DECIMALS)),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    test_instructions::cancel_stop_loss(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Martin: set SL on the position
    test_instructions::set_stop_loss_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        SetStopLossShortParams {
            stop_loss_limit_price: utils::scale(120, Cortex::PRICE_DECIMALS),
            close_position_price: Some(utils::scale(141, Cortex::PRICE_DECIMALS)),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Stop loss fails when limit is not reached
    {
        test_setup
            .update_oracle_price("sol", utils::scale(110, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 10).await;

        // Trigger SL automation (kickoff fails)
        assert!(!utils::execute_stop_loss_short_automation(
            &test_setup.program_test_ctx,
            &position_pda,
            &test_setup.payer_keypair,
        )
        .await
        .unwrap());
    }

    // Trigger Stop Loss
    {
        println!("Starting stop loss trigger test...");

        // First update: Set price just below trigger
        test_setup
            .update_oracle_price("sol", utils::scale(119, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 3).await;
        println!("Updated price to 119, warming up...");

        // Second update: Move price above trigger
        test_setup
            .update_oracle_price("sol", utils::scale(122, Cortex::PRICE_DECIMALS), 0)
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 3).await;
        println!("Updated price to 122, triggering stop loss...");

        // Clear any pending transactions
        let mut ctx = test_setup.program_test_ctx.write().await;
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        drop(ctx);

        // Trigger SL automation with fresh state
        let result = utils::execute_stop_loss_short_automation(
            &test_setup.program_test_ctx,
            &position_pda,
            &test_setup.payer_keypair,
        )
        .await;

        match result {
            Ok(executed) => {
                if !executed {
                    println!("Stop loss automation returned false");
                }
                assert!(executed, "Stop loss automation should succeed");
            }
            Err(e) => {
                println!("Stop loss execution failed with error: {:?}", e);
                panic!("Stop loss automation failed: {:?}", e);
            }
        }
    }
}
