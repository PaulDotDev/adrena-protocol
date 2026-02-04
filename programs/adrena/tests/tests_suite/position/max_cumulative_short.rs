use {
    crate::{
        test_instructions,
        utils::{self, fixtures, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionShortParams},
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

const BTC_DECIMALS: u8 = 6;
const USDC_DECIMALS: u8 = 6;

pub async fn max_cumulative_short() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(100000000, USDC_DECIMALS),
                    "btc" => utils::scale(10000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1000000, USDC_DECIMALS),
                    "btc" => utils::scale(1000000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "john",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1000000, USDC_DECIMALS),
                    "btc" => utils::scale(1000000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "george",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1000000, USDC_DECIMALS),
                    "btc" => utils::scale(1000000, BTC_DECIMALS),
                },
            },
        ],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
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
                        max_cumulative_short_position_size_usd: 0,
                        ..fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10000000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "btc",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(100.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(30000, Cortex::PRICE_DECIMALS),
                        initial_conf: 300000000000, // 10 bps
                        oracle_name: LimitedString::new("btc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::BTC,
                    },
                    trade_oracle: None,
                    pricing_params: Some(PricingParams {
                        max_cumulative_short_position_size_usd: utils::scale(
                            500000,
                            Cortex::USD_DECIMALS,
                        ),
                        ..fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(333, BTC_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(1000000, Cortex::LM_DECIMALS),
        utils::scale(1000000, Cortex::LM_DECIMALS),
        utils::scale(1000000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");
    let john = test_setup.get_user_keypair_by_name("john");
    let george = test_setup.get_user_keypair_by_name("george");

    let btc_mint = &test_setup.get_mint_by_name("btc");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    {
        let names = ["martin", "john", "george"];
        for (i, user) in [martin, john, george].iter().enumerate() {
            test_instructions::init_user_profile(
                &test_setup.program_test_ctx,
                user.pubkey(),
                user,
                &test_setup.payer_keypair,
                InitUserProfileParams {
                    nickname: names[i].to_string(),
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
    }

    // More than max cumulative short should fail
    assert!(test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(200000, USDC_DECIMALS),
            leverage: 30000, // x3
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());

    test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(50000, USDC_DECIMALS),
            leverage: 40000, // x4
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        john,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(50000, USDC_DECIMALS),
            leverage: 40000, // x4
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // More than max cumulative short should fail
    assert!(test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        george,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(50000, USDC_DECIMALS),
            leverage: 30000, // x3
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());

    test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        george,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(50000, USDC_DECIMALS),
            leverage: 20000, // x2
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // More than max cumulative short should fail
    assert!(test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        george,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        btc_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(5000, USDC_DECIMALS),
            leverage: 20000, // x2
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());
}
