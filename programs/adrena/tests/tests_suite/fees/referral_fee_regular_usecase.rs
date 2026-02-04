use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            ClosePositionLongParams, InitUserProfileParams, OpenPositionWithSwapParams,
        },
        state::{
            cortex::Cortex,
            user_profile::{Continent, ProfilePicture, Team, Title, UserProfile, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;
const BTC_DECIMALS: u8 = 6;

pub async fn referral_fee_regular_usecase() {
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
        Some("paul"),
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let paul = test_setup.get_user_keypair_by_name("paul");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let btc_mint = &test_setup.get_mint_by_name("btc");

    let alice_usdc_ata = utils::find_associated_token_account(
        &alice.pubkey(),
        &test_setup.get_fee_redistribution_mint(),
    )
    .0;

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let alice_user_profile_pda = {
        let alice_user_profile_pda = test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            alice.pubkey(),
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "alice".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            None,
        )
        .await
        .unwrap()
        .0;

        // Alice is Paul's referrer
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            paul.pubkey(),
            paul,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "paul".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            Some(alice_user_profile_pda),
        )
        .await
        .unwrap();

        alice_user_profile_pda
    };

    // Paul makes a trade, which should reward Alice
    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            paul,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            btc_mint,
            OpenPositionWithSwapParams {
                // Amount of ETH to use as collateral
                // ~$1000 of collateral
                collateral: 900000000,
                leverage: 40_000, // x4
                // max price paid for BTC when opening the position (slippage implied)
                price: utils::scale(30_400, Cortex::PRICE_DECIMALS),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap()
        .0;

        utils::warp_forward(
            &test_setup.program_test_ctx,
            adrena::state::position::MIN_POSITION_OPEN_TIME_SECONDS as i64,
        )
        .await;

        test_instructions::close_position_long(
            &test_setup.program_test_ctx,
            paul,
            &test_setup.payer_keypair,
            &position_pda,
            ClosePositionLongParams {
                // lowest exit price paid (slippage implied)
                price: Some(utils::scale(28_000, USDC_DECIMALS)),
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
                percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;
    }

    // Verify the referrer has rewards to claim
    {
        let alice_user_profile =
            utils::get_account::<UserProfile>(&test_setup.program_test_ctx, alice_user_profile_pda)
                .await;

        assert_eq!(alice_user_profile.claimable_referral_fee_usd, 858000);
    }

    // Claim the rewards
    {
        let alice_usdc_ata_balance_before =
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

        test_instructions::claim_referral_fee(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let alice_usdc_ata_balance_after =
            utils::get_token_account_balance(&test_setup.program_test_ctx, alice_usdc_ata).await;

        assert_eq!(
            alice_usdc_ata_balance_after - alice_usdc_ata_balance_before,
            858000
        );
    }

    // Make sure the accounting is all good
    {
        let alice_user_profile =
            utils::get_account::<UserProfile>(&test_setup.program_test_ctx, alice_user_profile_pda)
                .await;

        assert_eq!(alice_user_profile.claimable_referral_fee_usd, 0);

        let referrer_reward_token_vault_pda =
            pda::get_referrer_reward_token_vault(&test_setup.get_fee_redistribution_mint()).0;

        // Referrer vault should be empty now
        assert!(
            utils::get_token_account_balance(
                &test_setup.program_test_ctx,
                referrer_reward_token_vault_pda
            )
            .await
                == 0
        );
    }
}
