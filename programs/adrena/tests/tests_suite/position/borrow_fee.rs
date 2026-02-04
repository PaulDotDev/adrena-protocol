use {
    crate::{
        test_instructions::{self, open_position_long},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            ClosePositionLongParams, IncreasePositionLongParams, InitUserProfileParams,
            OpenPositionLongParams,
        },
        state::{
            cortex::Cortex,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const BONK_DECIMALS: u8 = 5;
const USDC_DECIMALS: u8 = 6;

pub async fn borrow_fee() {
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
                    pricing_params: None,
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
            collateral: utils::scale(1000000, BONK_DECIMALS),
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

        assert_eq!(pnl.borrow_fee_usd, 4247);
    }

    // Double the size of the position
    // User must pay borrow fee at increase time
    test_instructions::increase_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        bonk_mint,
        IncreasePositionLongParams {
            price: 190000,
            collateral: utils::scale(1000000, BONK_DECIMALS),
            leverage: 50000, // x5
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

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

        // The borrow fee shouldn't have shifted
        assert_eq!(pnl.borrow_fee_usd, 4247);
    }

    {
        let position = utils::get_zero_copy_account::<adrena::state::position::Position>(
            &test_setup.program_test_ctx,
            position_pda,
        )
        .await;

        // Must be 2M BONK
        assert_eq!(
            position.collateral_amount,
            utils::scale(2000000, BONK_DECIMALS)
        );

        // Borrow fee accrued prior to position increase
        assert_eq!(position.unrealized_interest_usd, 4247);
    }

    // Wait for 10 hours so we can see the borrow rate in action
    utils::warp_forward(&test_setup.program_test_ctx, 36000).await;

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

        // 4249 + 16999 (2 times amount + 2 times rate)
        assert_eq!(pnl.borrow_fee_usd, 21237);
    }

    let martin_bonk_ata_balance_before =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_bonk_ata).await;

    test_instructions::close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            price: Some(170000),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    let martin_bonk_ata_balance_after =
        utils::get_token_account_balance(&test_setup.program_test_ctx, martin_bonk_ata).await;

    // Fees (exit + borrow + borrow_prior_to_increase)
    assert_eq!(
        martin_bonk_ata_balance_after - martin_bonk_ata_balance_before,
        utils::scale(2000000, BONK_DECIMALS) - 2715971515
    );
}
