use {
    crate::{
        test_instructions,
        utils::{self, warp_forward, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionShortParams},
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

const ETH_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn single_short() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(200_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                },
            },
        ],
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
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(15.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1500, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("eth"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::ETH,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: Some(utils::fixtures::no_fees()),
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(10, ETH_DECIMALS),
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

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

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

    // Martin: Open 1 ETH short position x2
    test_instructions::open_position_short(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        usdc_mint,
        OpenPositionShortParams {
            // max price paid (slippage implied)
            price: utils::scale(1_450, Cortex::PRICE_DECIMALS),
            collateral: utils::scale(1_500, USDC_DECIMALS),
            leverage: 20_000,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Value after add liquidity
    let aum_after_init: u128 = 29962500000;

    let aum_usd_after_open_long_position =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    assert_eq!(aum_after_init - aum_usd_after_open_long_position, 0);

    warp_forward(&test_setup.program_test_ctx, 3_600).await;

    let aum_usd_after_open_long_position_after_warp =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    // Check ETH custody stats
    {
        let eth_custody: Custody = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            test_setup.custodies_info[1].custody_pda,
        )
        .await;

        let short_stats = eth_custody.short_positions;

        assert_eq!(eth_custody.assets.owned, utils::scale(10, ETH_DECIMALS));
        assert_eq!(short_stats.borrow_size_usd, 0);
        assert_eq!(short_stats.cumulative_interest_usd, 0);
        assert_eq!(short_stats.cumulative_interest_snapshot.to_u128(), 0);
        assert_eq!(short_stats.locked_amount, 0);
        assert_eq!(short_stats.open_positions, 1);
        assert_eq!(short_stats.size_usd, 3000000000);
        assert_eq!(short_stats.total_quantity.to_u128(), 20000);
    }

    // Check USDC custody stats
    {
        let usdc_custody: Custody = utils::get_zero_copy_account::<Custody>(
            &test_setup.program_test_ctx,
            test_setup.custodies_info[0].custody_pda,
        )
        .await;

        let short_stats = usdc_custody.short_positions;

        assert_eq!(
            usdc_custody.assets.owned,
            utils::scale(15_000, USDC_DECIMALS)
        );
        assert_eq!(short_stats.borrow_size_usd, 3000000000);
        assert_eq!(short_stats.cumulative_interest_usd, 0);
        assert_eq!(short_stats.cumulative_interest_snapshot.to_u128(), 0);
        assert_eq!(
            short_stats.locked_amount,
            utils::scale(3_000, USDC_DECIMALS)
        );
        assert_eq!(short_stats.open_positions, 1);
        assert_eq!(short_stats.size_usd, 0);
        assert_eq!(short_stats.total_quantity.to_u128(), 0);
    }

    // AUM increased due to position fees
    assert_eq!(
        aum_usd_after_open_long_position_after_warp - aum_usd_after_open_long_position,
        60000
    );

    // Makes ETH price to raise 10%
    test_setup
        .update_oracle_price("eth", utils::scale(1_650, Cortex::PRICE_DECIMALS), 0)
        .await;

    warp_forward(&test_setup.program_test_ctx, 3_600).await;

    let aum_usd_after_open_long_position_after_price_increase =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    // When the price raised from 10%, user lose money that is accounted for in the PnL, thus in the assets under management
    //
    // change comes from the interest paid + user PnL (money loss + exit fee)

    // Asset under management price change explained:
    //
    // 10 ETH price changed (+10%) => +$1,500 (number of ETH owned by the pool)
    // User pay: $X in interest for 2h
    // User lost: $300 (entry price vs exit price)
    //
    assert_eq!(
        aum_usd_after_open_long_position_after_price_increase - aum_after_init,
        1837620000,
    );

    // Makes ETH price to drop 80%
    test_setup
        .update_oracle_price("eth", utils::scale(330, Cortex::PRICE_DECIMALS), 0)
        .await;

    warp_forward(&test_setup.program_test_ctx, 3_600).await;

    // The idea here is to check that user gains are capped correctly
    //
    // The max user gains is 50% of position size: $742.5, thus max decrease of aum is $742.5 minus fees
    //
    // This test have been added following a bug in the initial implementation where the gains wasn't capped when shorting (accounting bug)

    let aum_usd_after_open_long_position_after_price_decrease =
        utils::get_assets_under_management_usd(&test_setup.program_test_ctx, test_setup.pool_pda)
            .await
            .unwrap();

    // When the price crash of 65% (total), user gains money, thus the assets under management of the pool decrease
    //
    // change comes from the interest paid + user PnL (user money gain - exit fee)

    // Asset under management price change explained:
    //
    // 10 ETH price changed (-65%) => -$11,700
    // User paid: $0.011025 in interest for 3h
    // User potential gains: $1,151.700 (entry price: $1,485 vs exit price: $333.300)
    // User capped gains: $742.5 (capped at locked USDC => $742.5)
    //
    assert_eq!(
        aum_after_init - aum_usd_after_open_long_position_after_price_decrease,
        14002320000,
    );
}
