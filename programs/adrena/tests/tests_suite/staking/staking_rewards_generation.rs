use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidityParams, AddLockedStakeParams, AddVestParams, BucketName,
            ClosePositionLongParams, InitUserProfileParams, OpenPositionLongParams,
            RemoveLiquidityParams, SwapParams,
        },
        state::{
            cortex::Cortex,
            staking::StakingRound,
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn staking_rewards_generation() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
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
                liquidity_amount: utils::scale(1_500, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
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
                liquidity_amount: utils::scale(1, ETH_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        Some("martin"),
    )
    .await;

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lm_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");

    let eth_mint = &test_setup.get_mint_by_name("eth");
    let usdc_mint = &test_setup.get_mint_by_name("usdc");

    test_instructions::init_user_staking(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &lm_token_mint_pda,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Prep work: Alice get 2000 governance tokens using vesting
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            alice,
            &AddVestParams {
                amount: utils::scale(2000, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        // Move until vest end
        utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

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

    // Create locked stake to make instructions to generate fees for LM locked stakers
    {
        test_instructions::add_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLockedStakeParams {
                amount: utils::scale(1500, Cortex::LM_DECIMALS),
                locked_days: 180,
            },
            &lm_token_mint_pda,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();

        utils::warp_forward(
            &test_setup.program_test_ctx,
            StakingRound::ROUND_MIN_DURATION_SECONDS,
        )
        .await;

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Check that add liquidity generates rewards
    {
        let lm_staking_reward_token_account_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Generate platform activity to fill current round' rewards
        test_instructions::add_liquidity(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            AddLiquidityParams {
                amount_in: 250000000,
                min_lp_amount_out: 1,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let lm_staking_reward_token_account_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Check rewards has been generated
        assert_eq!(
            lm_staking_reward_token_account_balance_after
                - lm_staking_reward_token_account_balance_before,
            75000,
        );
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check that open position generates rewards
    let position_pda = {
        let lm_staking_reward_token_account_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Martin: Open 0.1 ETH long position x1
        let position_pda = test_instructions::open_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            OpenPositionLongParams {
                // max price paid (slippage implied)
                price: utils::scale(1_550, Cortex::PRICE_DECIMALS),
                collateral: 100000000,
                leverage: 11_000, // x1.1
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

        let lm_staking_reward_token_account_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // No staking reward yet as open position does not pay fees
        assert_eq!(
            lm_staking_reward_token_account_balance_after
                - lm_staking_reward_token_account_balance_before,
            0
        );

        position_pda
    };

    utils::warp_forward(
        &test_setup.program_test_ctx,
        adrena::state::position::MIN_POSITION_OPEN_TIME_SECONDS as i64,
    )
    .await;

    // Check that close position generates rewards
    {
        let lm_staking_reward_token_account_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Martin: Close the ETH position
        test_instructions::close_position_long(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &position_pda,
            ClosePositionLongParams {
                // lowest exit price paid (slippage implied)
                price: Some(utils::scale(1_485, USDC_DECIMALS)),
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

        let lm_staking_reward_token_account_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Check rewards has been generated
        assert_eq!(
            lm_staking_reward_token_account_balance_after
                - lm_staking_reward_token_account_balance_before,
            52802,
        );
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check that swap doesn't generates rewards (feeless)
    {
        let lm_staking_reward_token_account_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Martin: Swap 150 USDC for ETH
        test_instructions::swap(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            // The program receives USDC
            usdc_mint,
            SwapParams {
                amount_in: utils::scale(150, USDC_DECIMALS),

                // 1% slippage
                min_amount_out: utils::scale(150, USDC_DECIMALS)
                    / utils::scale(1_500, ETH_DECIMALS)
                    * 99
                    / 100,

                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let lm_staking_reward_token_account_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Check rewards has been generated
        assert_eq!(
            lm_staking_reward_token_account_balance_after
                - lm_staking_reward_token_account_balance_before,
            0,
        );
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check that remove liquidity generates rewards
    {
        let lm_staking_reward_token_account_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Generate platform activity to fill current round' rewards
        test_instructions::remove_liquidity(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            RemoveLiquidityParams {
                lp_amount_in: utils::scale(1, Cortex::LP_DECIMALS),
                min_amount_out: 0,
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        let lm_staking_reward_token_account_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        // Check rewards has been generated
        assert_eq!(
            lm_staking_reward_token_account_balance_after
                - lm_staking_reward_token_account_balance_before,
            200,
        );
    }
}
