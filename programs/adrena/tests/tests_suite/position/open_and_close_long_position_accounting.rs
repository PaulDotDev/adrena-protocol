use {
    crate::{
        assert_unchanged,
        test_instructions::{self, close_position_long, open_position_long},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            ClosePositionLongParams, InitUserProfileParams, OpenPositionLongParams,
            ResolvePositionBorrowFeesParams,
        },
        state::{
            cortex::Cortex,
            custody::{Custody, PricingParams},
            pool::Pool,
            position::{Position, Side},
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::{limited_string::LimitedString, u128_split::U128Split},
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const JITOSOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

#[allow(deprecated)]
pub async fn open_and_close_long_position_accounting() {
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

    let martin_jitosol_ata =
        utils::find_associated_token_account(&martin.try_pubkey().unwrap(), jitosol_mint).0;

    let jitosol_custody_pda = test_setup.custodies_info[1].custody_pda;

    let jitosol_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, jitosol_custody_pda)
            .await;

    let martin_jitosol_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata).await;

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

    // Martin: Open 2 SOL long position x5
    let position_pda = open_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        OpenPositionLongParams {
            // max price paid (slippage implied)
            price: utils::scale(110, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1, JITOSOL_DECIMALS),
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
        let jitosol_custody_account_after = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;
        let martin_jitosol_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata)
                .await;

        // Check user balance
        {
            // Paid 1 jitoSOL as collateral
            assert_eq!(
                martin_jitosol_ata_balance_before - martin_jitosol_ata_balance_after,
                1000000000,
            );
        }

        // Check the position PDA info
        {
            let position = utils::get_zero_copy_account::<Position>(
                &test_setup.program_test_ctx,
                position_pda,
            )
            .await;

            assert_eq!(position.get_side(), Side::Long);
            // entry price
            // price of the token
            assert_eq!(position.price, 1000000000000);
            assert_eq!(position.size_usd, 997500000);
            assert_eq!(position.borrow_size_usd, 997500000);
            // 1 jitoSOL at price
            assert_eq!(position.collateral_usd, 199500000);
            // 1 jitoSOL
            assert_eq!(position.collateral_amount, 1000000000);
            assert_eq!(position.unrealized_interest_usd, 0);
            assert_eq!(position.cumulative_interest_snapshot, U128Split::from(0));
            assert_eq!(position.locked_amount, 5000000000);

            assert_eq!(position.exit_fee_usd, 1600000);
            assert_eq!(position.liquidation_fee_usd, 1600000);
        }

        // Double check effect of opening position on ETH custody accounting
        {
            // Collected fees
            {
                let before = &jitosol_custody_account_before.collected_fees;
                let after = &jitosol_custody_account_after.collected_fees;

                assert_unchanged!(before.swap_usd, after.swap_usd);
                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.close_position_usd, after.close_position_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Volume stats
            {
                let before = &jitosol_custody_account_before.volume_stats;
                let after = &jitosol_custody_account_after.volume_stats;

                assert_eq!(
                    after.open_position_usd - before.open_position_usd,
                    997500000,
                );

                assert_eq!(after.swap_usd - before.swap_usd, 0); // TODO should not be 0 (was 2250000)

                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.close_position_usd, after.close_position_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Trade Stats
            {
                let before = &jitosol_custody_account_before.trade_stats;
                let after = &jitosol_custody_account_after.trade_stats;

                assert_eq!(after.oi_long_usd - before.oi_long_usd, 997500000);

                assert_unchanged!(before.profit_usd, after.profit_usd);
                assert_unchanged!(before.oi_short_usd, after.oi_short_usd);
            }

            // Long positions
            {
                let before = &jitosol_custody_account_before.long_positions;
                let after = &jitosol_custody_account_after.long_positions;

                assert_eq!(after.open_positions - before.open_positions, 1);

                assert_eq!(after.size_usd - before.size_usd, 997500000);

                assert_eq!(after.borrow_size_usd - before.borrow_size_usd, 997500000);

                assert_eq!(after.locked_amount - before.locked_amount, 5000000000);

                assert_eq!(
                    after.total_quantity - before.total_quantity,
                    U128Split::from(99750)
                );

                // WeightedPrice = position_price * quantity
                assert_eq!(
                    after.weighted_price - before.weighted_price,
                    U128Split::from(99750000000000000_u128)
                );

                assert_unchanged!(
                    before.cumulative_interest_usd,
                    after.cumulative_interest_usd
                );

                assert_unchanged!(
                    before.cumulative_interest_snapshot,
                    after.cumulative_interest_snapshot
                );
            }

            // Short positions
            {
                let before = &jitosol_custody_account_before.short_positions;
                let after = &jitosol_custody_account_after.short_positions;

                assert_unchanged!(before.open_positions, after.open_positions);
                assert_unchanged!(before.size_usd, after.size_usd);
                assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                assert_unchanged!(before.locked_amount, after.locked_amount);
                assert_unchanged!(before.weighted_price, after.weighted_price);
                assert_unchanged!(before.total_quantity, after.total_quantity);
                assert_unchanged!(
                    before.cumulative_interest_usd,
                    after.cumulative_interest_usd
                );
                assert_unchanged!(
                    before.cumulative_interest_snapshot,
                    after.cumulative_interest_snapshot
                );
            }
        }
    }

    // Wait for 10 hours so we can see the borrow rate in action
    utils::warp_forward(&test_setup.program_test_ctx, 36000).await;

    // Makes SOL price to drop 10%
    test_setup
        .update_oracle_price(
            "sol",
            utils::scale(90, Cortex::PRICE_DECIMALS),
            900000000, // 10 bps
        )
        .await;

    let jitosol_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, jitosol_custody_pda)
            .await;

    let martin_jitosol_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata).await;

    // Adding this here (shouldn't have any impact on the tests bellow)
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

    // Martin: Close the SOL position
    close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            price: Some(utils::scale(80, Cortex::PRICE_DECIMALS)),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    {
        let jitosol_custody_account_after = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;
        let martin_jitosol_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata)
                .await;

        // Check user balance
        {
            assert_eq!(
                martin_jitosol_ata_balance_after - martin_jitosol_ata_balance_before,
                489277431
            );
        }

        // Double check effect of closing position on jitoSOL custody accounting
        {
            // Collected fees
            {
                let before = &jitosol_custody_account_before.collected_fees;
                let after = &jitosol_custody_account_after.collected_fees;

                assert_eq!(
                    after.close_position_usd - before.close_position_usd,
                    1600000,
                );

                assert_eq!(after.borrow_usd - before.borrow_usd, 49875);

                assert_unchanged!(before.swap_usd, after.swap_usd);
                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Volume stats
            {
                let before = &jitosol_custody_account_before.volume_stats;
                let after = &jitosol_custody_account_after.volume_stats;

                // locked amount (size) * position price (entry price)
                assert_eq!(
                    after.close_position_usd - before.close_position_usd,
                    997500000,
                );

                assert_eq!(after.swap_usd - before.swap_usd, 0);
                assert_unchanged!(before.open_position_usd, after.open_position_usd);
                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Trade Stats
            {
                let before = &jitosol_custody_account_before.trade_stats;
                let after = &jitosol_custody_account_after.trade_stats;

                assert_eq!(after.loss_usd - before.loss_usd, 101399875);

                assert_eq!(after.oi_long_usd, 0);
                assert_unchanged!(before.profit_usd, after.profit_usd);
                assert_unchanged!(before.oi_short_usd, after.oi_short_usd);
            }

            // Long positions
            {
                let before = &jitosol_custody_account_before.long_positions;
                let after = &jitosol_custody_account_after.long_positions;

                assert_eq!(before.open_positions - 1, after.open_positions);

                // locked amount (size) * position price (entry price)
                assert_eq!(before.size_usd - after.size_usd, 997500000);

                // locked amount (size) * position price (entry price)
                assert_eq!(before.borrow_size_usd - after.borrow_size_usd, 997500000);

                assert_eq!(before.locked_amount - after.locked_amount, 5000000000);

                assert_eq!(
                    before.total_quantity - after.total_quantity,
                    U128Split::from(99750)
                );

                assert_eq!(
                    before.weighted_price - after.weighted_price,
                    U128Split::from(99750000000000000_u128)
                );

                assert_unchanged!(
                    before.cumulative_interest_snapshot,
                    after.cumulative_interest_snapshot
                );

                assert_unchanged!(
                    before.cumulative_interest_usd,
                    after.cumulative_interest_usd
                );
            }

            // Short positions
            {
                let before = &jitosol_custody_account_before.short_positions;
                let after = &jitosol_custody_account_after.short_positions;

                assert_unchanged!(before.open_positions, after.open_positions);
                assert_unchanged!(before.size_usd, after.size_usd);
                assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                assert_unchanged!(before.locked_amount, after.locked_amount);
                assert_unchanged!(before.weighted_price, after.weighted_price);
                assert_unchanged!(before.total_quantity, after.total_quantity);

                assert_unchanged!(
                    before.cumulative_interest_usd,
                    after.cumulative_interest_usd
                );

                assert_unchanged!(
                    before.cumulative_interest_snapshot,
                    after.cumulative_interest_snapshot
                );
            }
        }

        // Pool stats
        {
            let pool = utils::get_zero_copy_account::<Pool>(
                &test_setup.program_test_ctx,
                test_setup.pool_pda,
            )
            .await;

            assert_eq!(pool.fees_debt_usd, 0);
            assert_eq!(pool.referrers_fee_debt_usd, 0);
        }
    }
}
