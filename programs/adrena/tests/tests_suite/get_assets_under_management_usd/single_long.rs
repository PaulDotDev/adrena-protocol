use {
    crate::{
        test_instructions,
        utils::{self, warp_forward, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionLongParams},
        state::{
            cortex::Cortex,
            custody::Custody,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const JITO_SOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn single_long() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(200, JITO_SOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(200, JITO_SOL_DECIMALS),
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
                    target_ratio: utils::ratio_from_percentage(40.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    // Disable fees to simplify calculations
                    fees: Some(utils::fixtures::no_fees()),
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(15_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "jitoSOL",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(15.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(200, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("jitoSOL"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::JITOSOL,
                    },
                    trade_oracle: Some(SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("sol"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::SOL,
                    }),
                    pricing_params: None,
                    fees: Some(utils::fixtures::no_fees()),
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(150, JITO_SOL_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(200_000, Cortex::LM_DECIMALS),
        utils::scale(300_000, Cortex::LM_DECIMALS),
        utils::scale(500_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");

    let jitosol_mint = &test_setup.get_mint_by_name("jitoSOL");

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

    let aum_usd_before_open_long_position =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    // Martin: Open 1 ETH long position x1.1
    test_instructions::open_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        OpenPositionLongParams {
            // max price paid (slippage implied)
            price: utils::scale(120, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1, JITO_SOL_DECIMALS),
            leverage: 11_000,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    warp_forward(&test_setup.program_test_ctx, 1).await;

    // Update both jitoSOL and SOL price of 10%
    {
        test_setup
            .update_oracle_price("sol", utils::scale(110, Cortex::PRICE_DECIMALS), 0)
            .await;

        test_setup
            .update_oracle_price("jitoSOL", utils::scale(220, Cortex::PRICE_DECIMALS), 0)
            .await;
    }

    warp_forward(&test_setup.program_test_ctx, 1).await;

    let aum_usd_after_open_long_position =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    assert_eq!(
        aum_usd_after_open_long_position - aum_usd_before_open_long_position,
        // change comes from the assets appreciating of 10% minus the assets being locked in the long position
        3073555000
    );

    warp_forward(&test_setup.program_test_ctx, 3_600).await;

    let aum_usd_after_open_long_position_after_warp =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    // Check jitoSOL custody stats
    {
        let jitosol_custody: Custody = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            test_setup.custodies_info[1].custody_pda,
        )
        .await;

        let long_stats = jitosol_custody.long_positions;

        assert_eq!(
            jitosol_custody.assets.owned,
            utils::scale(150, JITO_SOL_DECIMALS)
        );
        assert_eq!(long_stats.borrow_size_usd, 219450000);
        assert_eq!(long_stats.cumulative_interest_usd, 0);
        assert_eq!(long_stats.cumulative_interest_snapshot.to_u128(), 0);
        assert_eq!(long_stats.locked_amount, 1100000000);
        assert_eq!(long_stats.open_positions, 1);
        assert_eq!(long_stats.size_usd, 219450000);
        assert_eq!(long_stats.total_quantity.to_u128(), 21945);
    }

    assert_eq!(
        aum_usd_after_open_long_position_after_warp,
        // change comes from the interest paid
        aum_usd_after_open_long_position + 161
    );
}
