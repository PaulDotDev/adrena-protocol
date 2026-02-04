use {
    crate::{
        test_instructions::{self},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, OpenPositionWithSwapParams},
        state::{
            cortex::Cortex,
            position::Position,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const BONK_DECIMALS: u8 = 5;
const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 6;

#[allow(deprecated)]
pub async fn increase_bonk_position() {
    let test_setup: utils::TestSetup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(50000000, USDC_DECIMALS),
                    "bonk" => utils::scale(10000000000000, BONK_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(200000, ETH_DECIMALS),
                    "bonk" => utils::scale(10000000000000, BONK_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(150000000, USDC_DECIMALS),
                    "eth"  => utils::scale(1000000, ETH_DECIMALS),
                    "bonk" => utils::scale(10000000000000, BONK_DECIMALS),
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
            utils::MintParam {
                name: "bonk",
                decimals: BONK_DECIMALS,
            },
        ],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(1000000000, Cortex::USD_DECIMALS),
        vec![
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "usdc",
                    is_stable: true,
                    target_ratio: utils::ratio_from_percentage(34.0),
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
                liquidity_amount: utils::scale(15000000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
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
                liquidity_amount: utils::scale(10000, ETH_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "bonk",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: 189512,
                        initial_conf: 359,
                        oracle_name: LimitedString::new("bonk"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::BONK,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(1000000000000, BONK_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        Some("paul"),
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");

    let usdc_mint = &test_setup.get_mint_by_name("usdc");
    let bonk_mint = &test_setup.get_mint_by_name("bonk");

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

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189512);

        test_setup.update_oracle_price("bonk", 189038, 359).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189274);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189195);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189155);
    }

    test_setup.update_oracle_price("bonk", 189512, 359).await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(600000, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189512);

        test_setup.update_oracle_price("bonk", 189038, 359).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(600000, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189275);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(600000, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189196);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(600000, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189156);
    }

    test_setup.update_oracle_price("bonk", 189512, 359).await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(10000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189510);

        test_setup.update_oracle_price("bonk", 189038, 359).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189467);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189431);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189400);
    }

    test_setup.update_oracle_price("bonk", 189512, 359).await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(100000000000, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189511);

        test_setup.update_oracle_price("bonk", 189038, 359).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(33333333333, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189392);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(33333333333, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189321);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            bonk_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(33333333333, BONK_DECIMALS),
                leverage: 20000, // x2
                price: 200000,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189273);
    }

    test_setup.update_oracle_price("bonk", 189512, 359).await;

    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(1000000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189510);

        test_setup.update_oracle_price("bonk", 189038, 359).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(100000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189467);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(100000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189431);

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        test_instructions::open_or_increase_position_with_swap_short(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            usdc_mint,
            bonk_mint,
            OpenPositionWithSwapParams {
                collateral: utils::scale(100000, USDC_DECIMALS),
                leverage: 20000, // x2
                price: 10,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        let position_account =
            utils::get_account::<Position>(&test_setup.program_test_ctx, position_pda).await;

        assert_eq!(position_account.price, 189401);
    }
}
