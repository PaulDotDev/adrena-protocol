use {
    crate::{
        assert_unchanged,
        test_instructions::{self, close_position_short, open_position_short},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{ClosePositionShortParams, InitUserProfileParams, OpenPositionShortParams},
        state::{
            cortex::Cortex,
            custody::{Custody, PricingParams},
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
pub async fn open_and_partial_close_short_position_accounting() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150000, USDC_DECIMALS),
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

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let jitosol_mint = &test_setup.get_mint_by_name("jitoSOL");

    let usdc_custody_pda = test_setup.custodies_info[0].custody_pda;
    let jitosol_custody_pda = test_setup.custodies_info[1].custody_pda;

    let martin_usdc_ata =
        utils::find_associated_token_account(&martin.try_pubkey().unwrap(), usdc_mint).0;
    let martin_jitosol_ata =
        utils::find_associated_token_account(&martin.try_pubkey().unwrap(), jitosol_mint).0;

    let jitosol_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, jitosol_custody_pda)
            .await;
    let usdc_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
            .await;

    let martin_usdc_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
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

    // Martin: Open SOL short with 500 USDC
    let position_pda = open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        usdc_mint,
        OpenPositionShortParams {
            // max price paid (slippage implied)
            price: utils::scale(90, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(500, USDC_DECIMALS),
            leverage: 30000, // x3
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // See impact of opening a new position
    {
        let jitosol_custody_account_after = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            jitosol_custody_pda,
        )
        .await;
        let usdc_custody_account_after =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;

        let martin_usdc_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
        let martin_jitosol_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata)
                .await;

        // Check user balance
        {
            // USDC
            assert_eq!(
                // Paid 500 USDC as collateral
                martin_usdc_ata_balance_before - martin_usdc_ata_balance_after,
                500000000,
            );

            // SOL
            assert_unchanged!(
                martin_jitosol_ata_balance_before,
                martin_jitosol_ata_balance_after
            );
        }

        // Check the position PDA info
        {
            let position = utils::get_zero_copy_account::<Position>(
                &test_setup.program_test_ctx,
                position_pda,
            )
            .await;

            assert_eq!(position.get_side(), Side::Short);
            // entry price
            assert_eq!(position.price, 1000000000000);
            assert_eq!(position.size_usd, 1498500000);
            assert_eq!(position.borrow_size_usd, 1498500000);
            assert_eq!(position.collateral_usd, 499500000);
            assert_eq!(position.collateral_amount, 500000000);
            assert_eq!(position.unrealized_interest_usd, 0);
            assert_eq!(position.cumulative_interest_snapshot, U128Split::from(0));

            // Locked amount in USDC
            assert_eq!(position.locked_amount, 1500000000);
        }

        // Double check effect of opening position on SOL/USDC custody accounting
        {
            // Collected fees
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.collected_fees;
                    let after = &jitosol_custody_account_after.collected_fees;

                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.collected_fees;
                    let after = &usdc_custody_account_after.collected_fees;

                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }
            }

            // Volume stats
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.volume_stats;
                    let after = &jitosol_custody_account_after.volume_stats;

                    assert_eq!(
                        after.open_position_usd - before.open_position_usd,
                        1498500000,
                    );

                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.volume_stats;
                    let after = &usdc_custody_account_after.volume_stats;

                    assert_unchanged!(before.open_position_usd, after.open_position_usd);
                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }
            }

            // Trade Stats
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.trade_stats;
                    let after = &jitosol_custody_account_after.trade_stats;

                    assert_eq!(after.oi_short_usd - before.oi_short_usd, 1498500000);

                    assert_unchanged!(before.oi_long_usd, after.oi_long_usd);
                    assert_unchanged!(before.profit_usd, after.profit_usd);
                    assert_eq!(before.loss_usd, 0);
                    assert_eq!(after.loss_usd, 0);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.trade_stats;
                    let after = &usdc_custody_account_after.trade_stats;

                    assert_unchanged!(before.oi_short_usd, after.oi_short_usd);
                    assert_unchanged!(before.oi_long_usd, after.oi_long_usd);
                    assert_unchanged!(before.profit_usd, after.profit_usd);
                    assert_unchanged!(before.loss_usd, after.loss_usd);
                }
            }

            // Long positions
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.long_positions;
                    let after = &jitosol_custody_account_after.long_positions;

                    assert_unchanged!(before.open_positions, after.open_positions);
                    assert_unchanged!(before.size_usd, after.size_usd);
                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(before.total_quantity, after.total_quantity);
                    assert_unchanged!(before.weighted_price, after.weighted_price);
                    assert_unchanged!(
                        before.cumulative_interest_usd,
                        after.cumulative_interest_usd
                    );
                    assert_unchanged!(
                        before.cumulative_interest_snapshot,
                        after.cumulative_interest_snapshot
                    );
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.long_positions;
                    let after = &usdc_custody_account_after.long_positions;

                    assert_unchanged!(before.open_positions, after.open_positions);
                    assert_unchanged!(before.size_usd, after.size_usd);
                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(before.total_quantity, after.total_quantity);
                    assert_unchanged!(before.weighted_price, after.weighted_price);
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

            // Short positions
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.short_positions;
                    let after = &jitosol_custody_account_after.short_positions;

                    assert_eq!(before.open_positions + 1, after.open_positions);
                    assert_eq!(after.size_usd - before.size_usd, 1498500000);
                    assert_eq!(
                        after.total_quantity - before.total_quantity,
                        U128Split::from(149850)
                    );

                    assert_eq!(
                        after.weighted_price - before.weighted_price,
                        U128Split::from(149850000000000000_u128)
                    );

                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(
                        before.cumulative_interest_usd,
                        after.cumulative_interest_usd
                    );
                    assert_unchanged!(
                        before.cumulative_interest_snapshot,
                        after.cumulative_interest_snapshot
                    );
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.short_positions;
                    let after = &usdc_custody_account_after.short_positions;

                    assert_eq!(after.open_positions - before.open_positions, 1);
                    assert_eq!(after.borrow_size_usd - before.borrow_size_usd, 1498500000);
                    assert_eq!(after.locked_amount - before.locked_amount, 1500000000);

                    assert_unchanged!(before.size_usd, after.size_usd);
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
    }

    // Wait for 10 hours so we can see the borrow rate in action
    utils::warp_forward(&test_setup.program_test_ctx, 36000).await;

    // Makes SOL price to drop 10%
    test_setup
        .update_oracle_price(
            "sol",
            utils::scale(90, Cortex::PRICE_DECIMALS),
            900000000, // 10bps
        )
        .await;

    let jitosol_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, jitosol_custody_pda)
            .await;
    let usdc_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
            .await;

    let martin_usdc_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
    let martin_jitosol_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata).await;

    // Martin: Partial close the SOL position
    close_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionShortParams {
            price: Some(utils::scale(100, Cortex::PRICE_DECIMALS)),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: (60 * Cortex::BPS_POWER) as u64,
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
        let usdc_custody_account_after =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, usdc_custody_pda)
                .await;

        let martin_usdc_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_usdc_ata).await;
        let martin_jitosol_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_jitosol_ata)
                .await;

        // Check user balance
        {
            // USDC
            assert_eq!(
                martin_usdc_ata_balance_after - martin_usdc_ata_balance_before,
                387773227,
            );

            // SOL
            assert_unchanged!(
                martin_jitosol_ata_balance_before,
                martin_jitosol_ata_balance_after
            );
        }

        // Double check effect of closing position on SOL custody accounting
        {
            // Collected fees
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.collected_fees;
                    let after = &jitosol_custody_account_after.collected_fees;

                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.collected_fees;
                    let after = &usdc_custody_account_after.collected_fees;

                    assert_eq!(
                        after.close_position_usd - before.close_position_usd,
                        1440000,
                    );
                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }
            }

            // Volume stats
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.volume_stats;
                    let after = &jitosol_custody_account_after.volume_stats;

                    assert_eq!(
                        after.close_position_usd - before.close_position_usd,
                        899100000,
                    );

                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.open_position_usd, after.open_position_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.volume_stats;
                    let after = &usdc_custody_account_after.volume_stats;

                    assert_unchanged!(before.close_position_usd, after.close_position_usd);
                    assert_unchanged!(before.swap_usd, after.swap_usd);
                    assert_unchanged!(before.open_position_usd, after.open_position_usd);
                    assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                    assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                    assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
                }
            }

            // Trade Stats
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.trade_stats;
                    let after = &jitosol_custody_account_after.trade_stats;

                    assert_unchanged!(before.oi_long_usd, after.oi_long_usd);
                    assert_unchanged!(before.loss_usd, after.loss_usd);
                    assert_eq!(before.oi_short_usd, 1498500000);
                    assert_eq!(after.oi_short_usd, 599400000);

                    assert_eq!(after.profit_usd - before.profit_usd, 88461001);
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.trade_stats;
                    let after = &usdc_custody_account_after.trade_stats;

                    assert_unchanged!(before.oi_long_usd, after.oi_long_usd);
                    assert_unchanged!(before.loss_usd, after.loss_usd);
                    assert_unchanged!(before.oi_short_usd, after.oi_short_usd);
                    assert_unchanged!(before.profit_usd, after.profit_usd);
                }
            }

            // Long positions
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.long_positions;
                    let after = &jitosol_custody_account_after.long_positions;

                    assert_unchanged!(before.open_positions, after.open_positions);
                    assert_unchanged!(before.size_usd, after.size_usd);
                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(before.total_quantity, after.total_quantity);
                    assert_unchanged!(before.weighted_price, after.weighted_price);
                    assert_unchanged!(
                        before.cumulative_interest_usd,
                        after.cumulative_interest_usd
                    );

                    assert_unchanged!(
                        before.cumulative_interest_snapshot,
                        after.cumulative_interest_snapshot
                    );
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.long_positions;
                    let after = &usdc_custody_account_after.long_positions;

                    assert_unchanged!(before.open_positions, after.open_positions);
                    assert_unchanged!(before.size_usd, after.size_usd);
                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(before.total_quantity, after.total_quantity);
                    assert_unchanged!(before.weighted_price, after.weighted_price);
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

            // Short positions
            {
                // SOL
                {
                    let before = &jitosol_custody_account_before.short_positions;
                    let after = &jitosol_custody_account_after.short_positions;

                    assert_unchanged!(before.open_positions, after.open_positions);
                    assert_eq!(before.size_usd - after.size_usd, 899100000);
                    assert_eq!(
                        before.weighted_price - after.weighted_price,
                        U128Split::from(89910000000000000_u128)
                    );
                    assert_eq!(
                        before.total_quantity - after.total_quantity,
                        U128Split::from(89910)
                    );

                    assert_unchanged!(before.borrow_size_usd, after.borrow_size_usd);
                    assert_unchanged!(before.locked_amount, after.locked_amount);
                    assert_unchanged!(
                        before.cumulative_interest_usd,
                        after.cumulative_interest_usd
                    );

                    assert_unchanged!(
                        before.cumulative_interest_snapshot,
                        after.cumulative_interest_snapshot
                    );
                }

                // USDC
                {
                    let before = &usdc_custody_account_before.short_positions;
                    let after = &usdc_custody_account_after.short_positions;

                    assert_unchanged!(before.open_positions - after.open_positions, 0);
                    assert_eq!(before.borrow_size_usd - after.borrow_size_usd, 899100000);
                    assert_eq!(before.locked_amount - after.locked_amount, 900000000);

                    assert_unchanged!(before.size_usd, after.size_usd);
                    assert_unchanged!(before.weighted_price, after.weighted_price);
                    assert_unchanged!(before.total_quantity, after.total_quantity);

                    assert_unchanged!(
                        after.cumulative_interest_usd - before.cumulative_interest_usd,
                        6000
                    );

                    assert_unchanged!(
                        after.cumulative_interest_snapshot - before.cumulative_interest_snapshot,
                        U128Split::from(10010)
                    );
                }
            }
        }
    }
}
