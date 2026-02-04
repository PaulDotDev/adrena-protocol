use {
    crate::{
        test_instructions::{self},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{IncreasePositionShortParams, OpenPositionShortParams},
        state::{cortex::Cortex, position::Position},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
};

const JITO_SOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn increase_position_short_issue_negative() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(1000000, JITO_SOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100000, JITO_SOL_DECIMALS),
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
                decimals: JITO_SOL_DECIMALS,
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
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0, // 10 bps
                        oracle_name: LimitedString::new("jitoSOL"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::JITOSOL,
                    },
                    trade_oracle: Some(SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0, // 10 bps
                        oracle_name: LimitedString::new("sol"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::SOL,
                    }),
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10_000, JITO_SOL_DECIMALS),
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

    let alice = test_setup.get_user_keypair_by_name("alice");

    let jitosol_mint = &test_setup.get_mint_by_name("jitoSOL");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    let position_pda = test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        usdc_mint,
        OpenPositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(100, USDC_DECIMALS),
            leverage: 50000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    {
        let position =
            utils::get_zero_copy_account::<Position>(&test_setup.program_test_ctx, position_pda)
                .await;
        println!(
            "@audit position {:#?} {:#?}",
            position.price, position.size_usd
        );
    }

    utils::warp_forward(&test_setup.program_test_ctx, 11).await;

    // Makes JITOSOL price increase of 20%
    test_setup
        .update_oracle_price("jitoSOL", utils::scale(120, Cortex::PRICE_DECIMALS), 0)
        .await;

    // Makes SOL price increase of 20%
    test_setup
        .update_oracle_price("sol", utils::scale(120, Cortex::PRICE_DECIMALS), 0)
        .await;

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let pnl = test_instructions::get_pnl(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        &position_pda,
        Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
    )
    .await
    .unwrap();

    assert_eq!(pnl.loss_usd, 100700000);
    assert_eq!(pnl.profit_usd, 0);
    assert_eq!(pnl.exit_fee_usd, 800000);

    test_instructions::increase_position_short(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        usdc_mint,
        IncreasePositionShortParams {
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(100, USDC_DECIMALS),
            leverage: 50000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    {
        let position =
            utils::get_zero_copy_account::<Position>(&test_setup.program_test_ctx, position_pda)
                .await;
        println!(
            "@audit position {:#?} {:#?}",
            position.price, position.size_usd
        );
    }

    let pnl = test_instructions::get_pnl(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        &position_pda,
        Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
    )
    .await
    .unwrap();

    assert_eq!(pnl.loss_usd, 101500001);
    assert_eq!(pnl.profit_usd, 0);
    assert_eq!(pnl.exit_fee_usd, 1600000);
}
