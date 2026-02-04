use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionShortParams},
        state::{
            cortex::Cortex,
            custody::BorrowRateParams,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const ETH_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn test_aum_manipulation() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(2_000_000, USDC_DECIMALS),
                    "eth" => utils::scale(2000, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
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
                        initial_conf: 0,
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    // Disable fees to simplify calculations
                    fees: Some(utils::fixtures::no_fees()),
                    borrow_rate: Some(BorrowRateParams {
                        max_hourly_borrow_interest_rate: 1000000000u64,
                    }), // set max hourly borrow rate
                },
                liquidity_amount: utils::scale(1_000_000, USDC_DECIMALS),
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
                        initial_conf: 0,
                        oracle_name: LimitedString::new("eth"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::ETH,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: Some(utils::fixtures::no_fees()),
                    borrow_rate: Some(BorrowRateParams {
                        max_hourly_borrow_interest_rate: 1000000000u64,
                    }), // set max hourly borrow rate
                },
                liquidity_amount: utils::scale(1000, ETH_DECIMALS),
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
    let alice = test_setup.get_user_keypair_by_name("alice");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    let aum_start = test_instructions::get_assets_under_management(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
    )
    .await
    .unwrap();

    {
        // Alice user profile initialization has been done in the setup by providing liquidity
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
        // Martin: Open 100k short position x4 for 1 hour to
        //         accumulate interest
        test_instructions::open_position_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            usdc_mint,
            OpenPositionShortParams {
                // max price paid (slippage implied)
                price: utils::scale(1_050, Cortex::PRICE_DECIMALS),
                collateral: utils::scale(100_000, USDC_DECIMALS),
                leverage: 40_000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 3000).await; // about 1 hour
    }

    {
        let aum_after_first_pos = test_instructions::get_assets_under_management(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        // new short position of alice
        test_instructions::open_position_short(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            usdc_mint,
            OpenPositionShortParams {
                // max price paid (slippage implied)
                price: utils::scale(1_050, Cortex::PRICE_DECIMALS),
                collateral: utils::scale(100_000, USDC_DECIMALS),
                leverage: 40_000, // x4
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let aum_after_second_pos = test_instructions::get_assets_under_management(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        // Makes ETH price to up 1%
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(1515, Cortex::PRICE_DECIMALS),
                0, // 0 bps
            )
            .await;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let aum_after_price_change = test_instructions::get_assets_under_management(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        assert_eq!(aum_start, 2496250000000);
        assert_eq!(aum_after_first_pos, 2629583333600);
        assert_eq!(aum_after_second_pos, 2629583333600);
        assert_eq!(aum_after_price_change, 2652723612000);
    }
}
