use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{state::cortex::Cortex, utils::limited_string::LimitedString},
    maplit::hashmap,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn lp_token_price() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {
                "usdc" => utils::scale(100_000, USDC_DECIMALS),
                "eth" => utils::scale(50, ETH_DECIMALS),
            },
        }],
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
                liquidity_amount: utils::scale(15_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
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
        ],
        utils::scale(1000000, Cortex::LM_DECIMALS),
        utils::scale(1000000, Cortex::LM_DECIMALS),
        utils::scale(1000000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    // Check LP token price after pool setup
    assert_eq!(
        test_instructions::get_lp_token_price(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
            Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        )
        .await
        .unwrap(),
        10014001235
    );

    // Increase asset price and check that lp token price increase
    {
        // Makes ETH price to increase of 10%
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(1_650, Cortex::PRICE_DECIMALS),
                16500000000, // 10 bps
            )
            .await;

        assert_eq!(
            test_instructions::get_lp_token_price(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                &test_setup.lp_token_mint_pda,
                Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            )
            .await
            .unwrap(),
            10522753888
        );
    }

    // Decrease asset price and check that lp token price decrease
    {
        // Makes ETH price to decrease of 20%
        test_setup
            .update_oracle_price(
                "eth",
                utils::scale(1_320, Cortex::PRICE_DECIMALS),
                13200000000, // 10 bps
            )
            .await;

        assert_eq!(
            test_instructions::get_lp_token_price(
                &test_setup.program_test_ctx,
                &test_setup.payer_keypair,
                &test_setup.pool_pda,
                &test_setup.lp_token_mint_pda,
                Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            )
            .await
            .unwrap(),
            9420055090
        );
    }
}
