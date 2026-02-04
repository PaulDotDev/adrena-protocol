use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::AddLiquidityParams, state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
};

const USDC_DECIMALS: u8 = 6;

pub async fn aum_soft_cap_usd() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {
                "usdc" => utils::scale(50_000_000, USDC_DECIMALS),
            },
        }],
        vec![utils::MintParam {
            name: "usdc",
            decimals: USDC_DECIMALS,
        }],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(10_000_000, Cortex::USD_DECIMALS),
        vec![utils::SetupCustodyWithLiquidityParams {
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
            liquidity_amount: utils::scale(1, USDC_DECIMALS),
            payer_user_name: "alice",
        }],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    // Add less than the AUM limit
    test_instructions::add_liquidity(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        usdc_mint,
        AddLiquidityParams {
            amount_in: utils::scale(4_999_999, USDC_DECIMALS),
            min_lp_amount_out: 1,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Add exactly up to the AUM limit
    test_instructions::add_liquidity(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        usdc_mint,
        AddLiquidityParams {
            amount_in: utils::scale(5_000_000, USDC_DECIMALS),
            min_lp_amount_out: 1,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Add more than the AUM limit should fail
    assert!(test_instructions::add_liquidity(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        usdc_mint,
        AddLiquidityParams {
            amount_in: utils::scale(100_000, USDC_DECIMALS),
            min_lp_amount_out: 1,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());

    // Increase the limit
    test_instructions::set_pool_aum_soft_cap_usd(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        utils::scale(20_000_000, Cortex::LM_DECIMALS),
    )
    .await
    .unwrap();

    // Add up to the new limit
    test_instructions::add_liquidity(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        usdc_mint,
        AddLiquidityParams {
            amount_in: utils::scale(10_000_000, USDC_DECIMALS),
            min_lp_amount_out: 1,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();
}
