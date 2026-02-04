use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddLiquidityParams, RemoveLiquidityParams},
        state::{cortex::Cortex, custody::Custody, pool::Pool},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
};

const USDC_DECIMALS: u8 = 6;

pub async fn fees() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {
                "usdc" => utils::scale(10000000, USDC_DECIMALS),
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
            liquidity_amount: utils::scale(0, USDC_DECIMALS),
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

    // Check add liquidity fee
    {
        let protocol_fee_recipient_account_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            test_setup.protocol_fee_recipient_usdc_token_account,
        )
        .await;

        test_instructions::add_liquidity(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            AddLiquidityParams {
                amount_in: utils::scale(1000000, USDC_DECIMALS),
                min_lp_amount_out: 1,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        {
            let pool_account = utils::get_zero_copy_account::<Pool>(
                &test_setup.program_test_ctx,
                test_setup.pool_pda,
            )
            .await;
            let custody_account = utils::get_zero_copy_account::<Custody>(
                &test_setup.program_test_ctx,
                test_setup.custodies_info[0].custody_pda,
            )
            .await;
            let protocol_fee_recipient_account_after = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                test_setup.protocol_fee_recipient_usdc_token_account,
            )
            .await;

            assert_eq!(pool_account.aum_usd.to_u128(), 998700300000);

            assert_eq!(custody_account.collected_fees.add_liquidity_usd, 1000000000);

            assert_eq!(
                protocol_fee_recipient_account_after - protocol_fee_recipient_account_before,
                100000000
            );

            assert_eq!(pool_account.fees_debt_usd, 0);
        }
    }

    // Check remove liquidity fee
    {
        let protocol_fee_recipient_account_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            test_setup.protocol_fee_recipient_usdc_token_account,
        )
        .await;

        test_instructions::remove_liquidity(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            RemoveLiquidityParams {
                lp_amount_in: utils::scale(100, Cortex::LP_DECIMALS),
                min_amount_out: 1,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        {
            let pool_account = utils::get_zero_copy_account::<Pool>(
                &test_setup.program_test_ctx,
                test_setup.pool_pda,
            )
            .await;
            let custody_account = utils::get_zero_copy_account::<Custody>(
                &test_setup.program_test_ctx,
                test_setup.custodies_info[0].custody_pda,
            )
            .await;
            let protocol_fee_recipient_account_after = utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                test_setup.protocol_fee_recipient_usdc_token_account,
            )
            .await;

            assert_eq!(pool_account.aum_usd.to_u128(), 998600299979);

            assert_eq!(custody_account.collected_fees.remove_liquidity_usd, 100171);

            assert_eq!(
                protocol_fee_recipient_account_after - protocol_fee_recipient_account_before,
                10018
            );

            assert_eq!(pool_account.fees_debt_usd, 0);
            assert_eq!(pool_account.referrers_fee_debt_usd, 0);
        }
    }
}
