use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{InitUserProfileParams, SwapParams},
        state::{
            cortex::Cortex,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;
const BTC_DECIMALS: u8 = 6;

pub async fn permissioned_swap() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc"  => utils::scale(10000000, USDC_DECIMALS),
                    "eth"  => utils::scale(10000, ETH_DECIMALS),
                    "btc"  => utils::scale(1000, BTC_DECIMALS),
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
                name: "btc",
                decimals: BTC_DECIMALS,
            },
        ],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        // +100k to account for add_liquidity fees
        utils::scale(10_100_000, Cortex::USD_DECIMALS),
        vec![
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "usdc",
                    is_stable: true,
                    target_ratio: utils::ratio_from_percentage(34.0),
                    min_ratio: utils::ratio_from_percentage(31.0),
                    max_ratio: utils::ratio_from_percentage(37.0),
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
                liquidity_amount: utils::scale(3400000, USDC_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
                    min_ratio: utils::ratio_from_percentage(30.0),
                    max_ratio: utils::ratio_from_percentage(36.0),
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
                liquidity_amount: utils::scale(2200, ETH_DECIMALS),
                payer_user_name: "martin",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "btc",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(33.0),
                    min_ratio: utils::ratio_from_percentage(30.0),
                    max_ratio: utils::ratio_from_percentage(36.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(30000, Cortex::PRICE_DECIMALS),
                        initial_conf: 300000000000, // 10 bps
                        oracle_name: LimitedString::new("btc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::BTC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(110, BTC_DECIMALS),
                payer_user_name: "martin",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        Some("alice"),
    )
    .await;

    let paul = test_setup.get_user_keypair_by_name("paul");
    let alice = test_setup.get_user_keypair_by_name("alice");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    {
        let names = ["alice", "paul"];
        for (i, user) in [alice, paul].iter().enumerate() {
            test_instructions::init_user_profile(
                &test_setup.program_test_ctx,
                user.pubkey(),
                user,
                &test_setup.payer_keypair,
                InitUserProfileParams {
                    nickname: names[i].to_string(),
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
    }

    // Swap with authorized user
    test_instructions::swap(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        btc_mint,
        SwapParams {
            amount_in: utils::scale(1, BTC_DECIMALS),
            min_amount_out: 0,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Swap with unauthorized user should fail
    assert!(test_instructions::swap(
        &test_setup.program_test_ctx,
        paul,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        btc_mint,
        SwapParams {
            amount_in: utils::scale(1, BTC_DECIMALS),
            min_amount_out: 0,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx,).await,
            ),
        },
    )
    .await
    .is_err());

    // Change whitelist
    test_instructions::set_pool_whitelisted_swapper(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        &paul.pubkey(),
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Swap with authorized user should succeed
    test_instructions::swap(
        &test_setup.program_test_ctx,
        paul,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        eth_mint,
        btc_mint,
        SwapParams {
            amount_in: utils::scale(1, BTC_DECIMALS),
            min_amount_out: 0,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();
}
