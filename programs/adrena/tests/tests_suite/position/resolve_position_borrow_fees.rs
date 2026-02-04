use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            ClosePositionLongParams, OpenPositionLongParams, ResolvePositionBorrowFeesParams,
        },
        state::{
            cortex::{Cortex, ProfitAndLoss},
            custody::{Custody, PricingParams},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const JITOSOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn resolve_position_borrow_fees() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(15000000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100000, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(15000000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100000, JITOSOL_DECIMALS),
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
                    pricing_params: None,
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
                    target_ratio: utils::ratio_from_percentage(100.0),
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
                        ..utils::fixtures::pricing_params_regular()
                    }),
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10000, JITOSOL_DECIMALS),
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

    let martin_jitosol_pda = utils::find_associated_token_account(&martin.pubkey(), jitosol_mint).0;

    let lp_token_mint_pda = pda::get_lp_token_mint_pda(&test_setup.pool_pda).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lp_staking_pda = pda::get_staking_pda(&lp_token_mint_pda).0;

    let lm_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;

    let lp_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lp_staking_pda).0;

    let jitosol_custody_pda = utils::get_custody_pda(&test_setup.pool_pda, jitosol_mint).0;
    let usdc_custody_pda = utils::get_custody_pda(&test_setup.pool_pda, usdc_mint).0;

    // Numbers out of the gate after setting up the pool
    let original_jitosol_custody_owned_amount = 10000000000000;
    let original_usdc_custody_owned_amount = 999100000000;

    {
        let jitosol_custody_account = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;

        let usdc_custody_account =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;

        assert_eq!(jitosol_custody_account.assets.collateral, 0);
        assert_eq!(
            jitosol_custody_account.assets.owned,
            original_jitosol_custody_owned_amount
        );
        assert_eq!(jitosol_custody_account.assets.locked, 0);

        assert_eq!(usdc_custody_account.assets.collateral, 0);
        assert_eq!(
            usdc_custody_account.assets.owned,
            original_usdc_custody_owned_amount
        );
        assert_eq!(usdc_custody_account.assets.locked, 0);
    }

    let initial_protocol_fee_recipient_usdc_token_account = utils::get_token_account_balance(
        &test_setup.program_test_ctx,
        test_setup.protocol_fee_recipient_usdc_token_account,
    )
    .await;

    let initial_aum_usd = test_instructions::get_assets_under_management(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        None,
    )
    .await
    .unwrap();

    let martin_jitosol_initial_balance =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_pda).await;

    // Martin: Open 1 JITOSOL long position x5
    let position_pda = test_instructions::open_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        OpenPositionLongParams {
            // max price paid (slippage implied)
            price: utils::scale(1_550, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1, JITOSOL_DECIMALS),
            leverage: 50_000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // Generate borrow fees
    utils::warp_forward(&test_setup.program_test_ctx, 700000).await;

    // Should be the same PnL BEFORE and AFTER
    let expected_pnl = ProfitAndLoss {
        borrow_fee_usd: 9698,
        profit_usd: 0,
        loss_usd: 1609698, // borrow_fee_usd + exit_fee_usd
        exit_fee: 8020050,
        exit_fee_usd: 1600000,
    };

    // Check the PnL prior to borrow payment
    {
        let pnl = test_instructions::get_pnl(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &position_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        assert_eq!(pnl.borrow_fee_usd, expected_pnl.borrow_fee_usd);
        assert_eq!(pnl.profit_usd, expected_pnl.profit_usd);
        assert_eq!(pnl.loss_usd, expected_pnl.loss_usd);
        assert_eq!(pnl.exit_fee, expected_pnl.exit_fee);
        assert_eq!(pnl.exit_fee_usd, expected_pnl.exit_fee_usd);
    }

    // Check the fees accounting in the custody BEFORE
    {
        let custody_account = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;

        assert_eq!(custody_account.collected_fees.borrow_usd, 0);
    }

    // Check the vault fees BEFORE
    let lm_staking_reward_token_vault_balance_before = utils::get_token_account_balance(
        &test_setup.program_test_ctx,
        lm_staking_reward_token_vault_pda,
    )
    .await;

    let lp_staking_reward_token_vault_balance_before = utils::get_token_account_balance(
        &test_setup.program_test_ctx,
        lp_staking_reward_token_vault_pda,
    )
    .await;

    test_instructions::resolve_position_borrow_fees(
        &test_setup.program_test_ctx,
        martin,
        &position_pda,
        ResolvePositionBorrowFeesParams {
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Check the PnL after borrow payment, should be the same
    {
        let pnl = test_instructions::get_pnl(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &position_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        assert_eq!(pnl.borrow_fee_usd, expected_pnl.borrow_fee_usd);
        assert_eq!(pnl.profit_usd, expected_pnl.profit_usd);
        assert_eq!(pnl.loss_usd, expected_pnl.loss_usd);
        assert_eq!(pnl.exit_fee, expected_pnl.exit_fee);
        assert_eq!(pnl.exit_fee_usd, expected_pnl.exit_fee_usd);
    }

    // Check the fees accounting in the custody AFTER
    let last_collected_borrow_fees = {
        let jitosol_custody_account = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;

        // Reflect the borrow fee usd paid
        assert_eq!(jitosol_custody_account.collected_fees.borrow_usd, 9698);
        assert_eq!(jitosol_custody_account.trade_stats.loss_usd, 9698);

        // Reflect the borrow fee usd paid
        // 70% of the borrow fee are collected to the pool -- doesn't mutate assets.owned
        // 30% of the borrow fee are swapped to USDC and sent to vaults
        {
            assert_eq!(
                jitosol_custody_account.assets.owned as i128
                    - original_jitosol_custody_owned_amount as i128,
                0 // 70% * borrow_fee_usd / jitosol_price
            );

            {
                let usdc_custody_account = utils::get_zero_copy_account::<Custody>(
                    &test_setup.program_test_ctx,
                    usdc_custody_pda,
                )
                .await;

                assert_eq!(
                    usdc_custody_account.assets.owned as i128
                        - original_usdc_custody_owned_amount as i128,
                    -2910 // 30% * borrow_fee_usd / usdc_price (rounded up)
                );
            }
        }

        jitosol_custody_account.collected_fees.borrow_usd
    };

    {
        let aum_usd = test_instructions::get_assets_under_management(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            None,
        )
        .await
        .unwrap();

        println!("AUM USD: {}", aum_usd);
        println!("INITIAL AUM USD: {}", initial_aum_usd);

        // AUM should grow by user PNL - borrow fee paid to external vaults
        // (pool pay borrow fees in advance)
        assert_eq!(
            aum_usd - initial_aum_usd,
            expected_pnl.loss_usd as u128 - 2910 + 2 // 2 due to rounding
        );
    }

    // Check the vault fees AFTER
    {
        let lm_staking_reward_token_vault_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        let lp_staking_reward_token_vault_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lp_staking_reward_token_vault_pda,
        )
        .await;

        let protocol_fee_recipient_usdc_token_account_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            test_setup.protocol_fee_recipient_usdc_token_account,
        )
        .await;

        // 20% share
        assert_eq!(
            lm_staking_reward_token_vault_balance_after
                - lm_staking_reward_token_vault_balance_before,
            1940
        );

        // No one is staked
        assert_eq!(
            lp_staking_reward_token_vault_balance_after
                - lp_staking_reward_token_vault_balance_before,
            0
        );

        // 10% share
        assert_eq!(
            protocol_fee_recipient_usdc_token_account_after
                - initial_protocol_fee_recipient_usdc_token_account,
            970
        );
    }

    // Generate new borrow fees
    utils::warp_forward(&test_setup.program_test_ctx, 500000).await;

    // Check the PnL prior to position close payment
    {
        let pnl = test_instructions::get_pnl(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &position_pda,
            Some(utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await),
        )
        .await
        .unwrap();

        assert_eq!(pnl.borrow_fee_usd, 16624); // With a part already paid
        assert_eq!(pnl.profit_usd, 0);
        assert_eq!(pnl.loss_usd, 1616624);
        assert_eq!(pnl.exit_fee, 8020050);
        assert_eq!(pnl.exit_fee_usd, 1600000);
    }

    // Martin: Close the JITOSOL position
    test_instructions::close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            // lower the price for slippage
            price: Some(utils::scale(90, Cortex::PRICE_DECIMALS)),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    {
        let jitosol_custody_account = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;

        let usdc_custody_account =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;

        assert_eq!(jitosol_custody_account.assets.collateral, 0);
        assert_eq!(
            jitosol_custody_account.assets.owned - original_jitosol_custody_owned_amount,
            // Loss USD as JITOSOL (At the end the pool is whole again - user repaid the borrow fees)
            13050494
        );
        assert_eq!(jitosol_custody_account.assets.locked, 0);

        assert_eq!(usdc_custody_account.assets.collateral, 0);
        assert_eq!(
            (usdc_custody_account.assets.owned as i128
                - original_usdc_custody_owned_amount as i128), // the original one, not the last one
            // 30% of the fees are paid in USDC
            // (Exit Fee + Borrow Fee) * 30%
            -484988
        );
        assert_eq!(usdc_custody_account.assets.locked, 0);
    }

    // Check the fees accounting in the custody AFTER
    {
        let custody_pda = utils::get_custody_pda(&test_setup.pool_pda, jitosol_mint).0;

        let custody_account =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, custody_pda)
                .await;

        assert_eq!(
            custody_account.collected_fees.borrow_usd - last_collected_borrow_fees,
            // Additional borrow fee
            6926
        );
    }

    // Check the vault fees AFTER
    {
        let lm_staking_reward_token_vault_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        let lp_staking_reward_token_vault_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lp_staking_reward_token_vault_pda,
        )
        .await;

        assert_eq!(
            lm_staking_reward_token_vault_balance_after
                - lm_staking_reward_token_vault_balance_before,
            // 20% * (Exit Fee + Total Borrow Fee)
            323325
        );

        // No one is staked
        assert_eq!(
            lp_staking_reward_token_vault_balance_after
                - lp_staking_reward_token_vault_balance_before,
            0
        );
    }

    // Check user account AFTER
    {
        let martin_jitosol_balance =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_pda)
                .await;

        assert_eq!(
            martin_jitosol_balance as i128 - martin_jitosol_initial_balance as i128,
            // Lost (Exit Fee + Borrow Fee)
            -13050494
        );
    }
}
