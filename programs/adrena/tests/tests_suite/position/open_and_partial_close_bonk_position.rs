use {
    crate::{
        assert_unchanged,
        test_instructions::{self, close_position_long, open_position_long},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{ClosePositionLongParams, InitUserProfileParams, OpenPositionLongParams},
        state::{
            cortex::Cortex,
            custody::{Custody, PricingParams},
            pool::Pool,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::{limited_string::LimitedString, u128_split::U128Split},
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const BONK_DECIMALS: u8 = 5;
const USDC_DECIMALS: u8 = 6;

#[allow(deprecated)]
pub async fn open_and_partial_close_bonk_position() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150000, USDC_DECIMALS),
                    "bonk" => utils::scale(1000000000, BONK_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150000, USDC_DECIMALS),
                    "bonk" => utils::scale(1000000000, BONK_DECIMALS),
                },
            },
        ],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
            },
            utils::MintParam {
                name: "bonk",
                decimals: BONK_DECIMALS,
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
                    mint_name: "bonk",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: 170329,
                        initial_conf: 334,
                        oracle_name: LimitedString::new("bonk"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::BONK,
                    },
                    trade_oracle: None,
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
                liquidity_amount: utils::scale(100000000, BONK_DECIMALS),
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

    let bonk_mint = &test_setup.get_mint_by_name("bonk");

    let martin_bonk_ata =
        utils::find_associated_token_account(&martin.try_pubkey().unwrap(), bonk_mint).0;

    let bonk_custody_pda = test_setup.custodies_info[1].custody_pda;

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

    // Martin: Open 1000k BONK long position x5
    let position_pda = open_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        bonk_mint,
        OpenPositionLongParams {
            // max price paid (slippage implied)
            price: 190000,
            collateral: utils::scale(10000000, BONK_DECIMALS),
            leverage: 50000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    // Wait for 10 hours so we can see the borrow rate in action
    utils::warp_forward(&test_setup.program_test_ctx, 36000).await;

    // Makes BONK price to drop 10%
    test_setup.update_oracle_price("bonk", 153296, 301).await;

    let bonk_custody_account_before =
        utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, bonk_custody_pda)
            .await;

    let martin_bonk_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_bonk_ata).await;

    // Martin: Partially close the BONK position
    close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            price: Some(80000),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: (60 * Cortex::BPS_POWER) as u64,
        },
    )
    .await
    .unwrap();

    {
        let bonk_custody_account_after =
            utils::get_zero_copy_account::<Custody>(&test_setup.program_test_ctx, bonk_custody_pda)
                .await;
        let martin_bonk_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_bonk_ata).await;

        // Check user balance
        {
            assert_eq!(
                martin_bonk_ata_balance_after - martin_bonk_ata_balance_before,
                324866136708
            );
        }

        // Double check effect of closing position on bonk custody accounting
        {
            // Collected fees
            {
                let before = &bonk_custody_account_before.collected_fees;
                let after = &bonk_custody_account_after.collected_fees;

                assert_eq!(after.close_position_usd - before.close_position_usd, 817580);

                assert_eq!(after.borrow_usd - before.borrow_usd, 254856);

                assert_unchanged!(before.swap_usd, after.swap_usd);
                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Volume stats
            {
                let before = &bonk_custody_account_before.volume_stats;
                let after = &bonk_custody_account_after.volume_stats;

                // locked amount (size) * position price (entry price)
                assert_eq!(
                    after.close_position_usd - before.close_position_usd,
                    509712000
                );

                assert_eq!(after.swap_usd - before.swap_usd, 0);
                assert_unchanged!(before.open_position_usd, after.open_position_usd);
                assert_unchanged!(before.add_liquidity_usd, after.add_liquidity_usd);
                assert_unchanged!(before.remove_liquidity_usd, after.remove_liquidity_usd);
                assert_unchanged!(before.liquidation_usd, after.liquidation_usd);
            }

            // Trade Stats
            {
                let before = &bonk_custody_account_before.trade_stats;
                let after = &bonk_custody_account_after.trade_stats;

                assert_eq!(after.loss_usd - before.loss_usd, 52043936);

                assert_eq!(after.oi_long_usd, 339808000);
                assert_unchanged!(before.profit_usd, after.profit_usd);
                assert_unchanged!(before.oi_short_usd, after.oi_short_usd);
            }

            // Long positions
            {
                let before = &bonk_custody_account_before.long_positions;
                let after = &bonk_custody_account_after.long_positions;

                // Did not change cause it's a partial close
                assert_eq!(before.open_positions, after.open_positions);

                // locked amount (size) * position price (entry price)
                assert_eq!(before.size_usd - after.size_usd, 509712000);

                // locked amount (size) * position price (entry price)
                assert_eq!(before.borrow_size_usd - after.borrow_size_usd, 509712000);

                assert_eq!(before.locked_amount - after.locked_amount, 3000000000000);

                assert_eq!(
                    before.total_quantity - after.total_quantity,
                    U128Split::from(299251448666_u64)
                );

                assert_eq!(
                    before.weighted_price - after.weighted_price,
                    U128Split::from(50971199999831114_u128)
                );

                assert_eq!(
                    after.cumulative_interest_snapshot - before.cumulative_interest_snapshot,
                    U128Split::from(500000_u128)
                );

                assert_eq!(before.cumulative_interest_usd, 0_u64);
            }

            // Short positions
            {
                let before = &bonk_custody_account_before.short_positions;
                let after = &bonk_custody_account_after.short_positions;

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

    // Martin: Close the rest of the position
    close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            price: Some(80000),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    {
        let martin_bonk_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, martin_bonk_ata).await;

        // Check user balance
        {
            assert_eq!(
                martin_bonk_ata_balance_after - martin_bonk_ata_balance_before,
                541443569860
            );
        }
    }
}
