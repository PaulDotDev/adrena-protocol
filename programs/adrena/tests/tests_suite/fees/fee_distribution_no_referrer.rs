use {
    crate::{
        test_instructions,
        utils::{self, fixtures, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{ClosePositionLongParams, DistributeFeesParams, OpenPositionWithSwapParams},
        state::{cortex::Cortex, custody::Fees, pool::Pool},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;
const BTC_DECIMALS: u8 = 6;

pub async fn fee_distribution_no_referrer() {
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
                    fees: Some(Fees {
                        add_liquidity: 0, // No fees so we start with a clean fee distribution after setup
                        fee_max: 0,
                        ..fixtures::fees_regular()
                    }),
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
                    fees: Some(Fees {
                        add_liquidity: 0, // No fees so we start with a clean fee distribution after setup
                        fee_max: 0,
                        ..fixtures::fees_regular()
                    }),
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
                    fees: Some(Fees {
                        add_liquidity: 0, // No fees so we start with a clean fee distribution after setup
                        fee_max: 0,
                        ..fixtures::fees_regular()
                    }),
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

    let eth_mint = &test_setup.get_mint_by_name("eth");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lp_staking_pda = pda::get_staking_pda(&test_setup.lp_token_mint_pda).0;
    let lm_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;
    let lp_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lp_staking_pda).0;

    let referrer_reward_token_vault_pda =
        pda::get_referrer_reward_token_vault(&test_setup.get_fee_redistribution_mint()).0;

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check pool to start with 0 fees
    {
        let pool_account =
            utils::get_zero_copy_account::<Pool>(&test_setup.program_test_ctx, test_setup.pool_pda)
                .await;

        assert_eq!(pool_account.fees_debt_usd, 0);
        assert_eq!(pool_account.referrers_fee_debt_usd, 0);
    }

    // Check what is in the vaults before generating fees
    {
        let lm_staking_reward_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        let lp_staking_reward_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lp_staking_reward_token_vault_pda,
        )
        .await;

        let referrer_vault_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            referrer_reward_token_vault_pda,
        )
        .await;

        let protocol_fee_recipient = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            test_setup.protocol_fee_recipient_usdc_token_account,
        )
        .await;

        assert_eq!(referrer_vault_token_account_balance, 0);
        assert_eq!(protocol_fee_recipient, 0);
        assert_eq!(lm_staking_reward_token_account_balance, 0);
        assert_eq!(lp_staking_reward_token_account_balance, 0);
    }

    // Generate fees to be redistributed
    {
        let position_pda = test_instructions::open_or_increase_position_with_swap_long(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            eth_mint,
            eth_mint,
            OpenPositionWithSwapParams {
                // Amount of ETH to use as collateral
                // ~$1000 of collateral
                collateral: 900000000,
                leverage: 40_000, // x4
                // max price paid for BTC when opening the position (slippage implied)
                price: utils::scale(1_501, Cortex::PRICE_DECIMALS),
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

        test_instructions::close_position_long_without_fee_distribution(
            &test_setup.program_test_ctx,
            alice,
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

    // Check pool after collecting fees
    {
        let pool_account =
            utils::get_zero_copy_account::<Pool>(&test_setup.program_test_ctx, test_setup.pool_pda)
                .await;

        assert_eq!(pool_account.fees_debt_usd, 2592002);
        assert_eq!(pool_account.referrers_fee_debt_usd, 0);
    }

    // Proceed with fee distribution
    {
        test_instructions::distribute_fees(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            &DistributeFeesParams {
                oracle_prices: Some(
                    utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx)
                        .await,
                ),
            },
        )
        .await
        .unwrap();
    }

    // Check pool after distributing fees
    {
        let pool_account =
            utils::get_zero_copy_account::<Pool>(&test_setup.program_test_ctx, test_setup.pool_pda)
                .await;

        assert_eq!(pool_account.fees_debt_usd, 0);
        assert_eq!(pool_account.referrers_fee_debt_usd, 0);
    }

    // Check what is in the vaults after fee distribution
    {
        let lm_staking_reward_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lm_staking_reward_token_vault_pda,
        )
        .await;

        let lp_staking_reward_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            lp_staking_reward_token_vault_pda,
        )
        .await;

        let referrer_vault_token_account_balance = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            referrer_reward_token_vault_pda,
        )
        .await;

        let protocol_fee_recipient = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            test_setup.protocol_fee_recipient_usdc_token_account,
        )
        .await;

        // Total fee: 8640005
        assert_eq!(referrer_vault_token_account_balance, 0);
        assert_eq!(protocol_fee_recipient, 864001); // Should be 10% of the total fee
        assert_eq!(lm_staking_reward_token_account_balance, 1728001); // Should be 20% of the total fee
        assert_eq!(lp_staking_reward_token_account_balance, 0); // Should be 0% of the total fee as there is no staked LP
    }
}
