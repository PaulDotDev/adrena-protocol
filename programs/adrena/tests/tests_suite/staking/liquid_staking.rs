use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidStakeParams, AddLiquidityParams, AddVestParams, BucketName, ClaimStakesParams,
            RemoveLiquidStakeParams,
        },
        state::{cortex::Cortex, staking::StakingRound},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn liquid_staking() {
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
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");

    let cortex_stake_reward_mint = test_setup.get_fee_redistribution_mint();

    let eth_mint = &test_setup.get_mint_by_name("eth");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    {
        // LM Staking
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();

        // LP Staking
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.lp_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();
    }

    // Prep work: Alice get 2 governance tokens using vesting
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            alice,
            &AddVestParams {
                amount: utils::scale(2, Cortex::LM_DECIMALS),
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

    let alice_staking_reward_token_account_address =
        utils::find_associated_token_account(&alice.pubkey(), &cortex_stake_reward_mint).0;

    let alice_lm_token_account_address =
        utils::find_associated_token_account(&alice.pubkey(), &lm_token_mint_pda).0;

    // Alice: add LM liquid staking
    {
        test_instructions::add_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLiquidStakeParams {
                amount: utils::scale(1, Cortex::LM_DECIMALS),
            },
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Alice: add LP liquid staking (should fail, there is no LP liquid staking)
    {
        assert!(test_instructions::add_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLiquidStakeParams {
                amount: utils::scale(1, Cortex::LP_DECIMALS),
            },
            &test_setup.governance_realm_pda,
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
        )
        .await
        .is_err());
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Alice: claim when there is nothing to claim yet
    {
        let balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &lm_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        let balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        assert_eq!(balance_before, balance_after);
        assert_eq!(lm_balance_before, lm_balance_after);
    }

    // warp to the next round and resolve the current one
    // this round bear no rewards for the new staking at the staking started during the round
    {
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

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.lp_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // warp to the next round and resolve the current one
    // this round bear rewards for the liquid stake
    {
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

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.lp_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Claim when there is one round worth of rewards to claim
    {
        let balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &lm_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        let balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        assert_eq!(balance_after - balance_before, 600000);
        // No lm reward from LP liquid stake (just rewarded from accruing value in the LP pool for all tokens)
        assert_eq!(lm_balance_after - lm_balance_before, 0);
    }

    // warp to the next round and resolve the current one
    {
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

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.lp_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Use add liquidity to generate rewards for the current round
    {
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
    }

    // Alice: add LM liquid stake when staking is already pending
    test_instructions::add_liquid_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        AddLiquidStakeParams {
            amount: utils::scale(1, Cortex::LM_DECIMALS),
        },
        &test_setup.governance_realm_pda,
        &test_setup.pool_pda,
        &lm_token_mint_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // warp to the next round and resolve the current one
    {
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

        test_instructions::resolve_staking_round(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.lp_token_mint_pda,
        )
        .await
        .unwrap();
    }

    // Claim rewards - alice should get rewards from her first liquid staking for this round
    // New staking should start accruing rewards next round only
    {
        let balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_before = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &lm_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        test_instructions::claim_stakes(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &alice.pubkey(),
            &test_setup.pool_pda,
            &test_setup.lp_token_mint_pda,
            &ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )
        .await
        .unwrap();

        let balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_staking_reward_token_account_address,
        )
        .await;

        let lm_balance_after = utils::get_token_account_balance(
            &test_setup.program_test_ctx,
            alice_lm_token_account_address,
        )
        .await;

        assert_eq!(balance_after - balance_before, 75000);
        assert_eq!(lm_balance_after - lm_balance_before, 0);
    }

    // Remove half the stake
    test_instructions::remove_liquid_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        RemoveLiquidStakeParams {
            amount: utils::scale(1, Cortex::LM_DECIMALS),
        },
        &cortex_stake_reward_mint,
        &test_setup.governance_realm_pda,
        &test_setup.pool_pda,
        &lm_token_mint_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Try & remove more than possible should fail
    assert!(test_instructions::remove_liquid_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        RemoveLiquidStakeParams {
            amount: utils::scale(42, Cortex::LM_DECIMALS),
        },
        &cortex_stake_reward_mint,
        &test_setup.governance_realm_pda,
        &test_setup.pool_pda,
        &lm_token_mint_pda,
    )
    .await
    .is_err());

    // Try & remove 0 tokens should fail
    assert!(test_instructions::remove_liquid_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        RemoveLiquidStakeParams { amount: 0 },
        &cortex_stake_reward_mint,
        &test_setup.governance_realm_pda,
        &test_setup.pool_pda,
        &lm_token_mint_pda,
    )
    .await
    .is_err());

    // Remove the other half of the stake
    test_instructions::remove_liquid_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        RemoveLiquidStakeParams {
            amount: utils::scale(1, Cortex::LM_DECIMALS),
        },
        &cortex_stake_reward_mint,
        &test_setup.governance_realm_pda,
        &test_setup.pool_pda,
        &lm_token_mint_pda,
    )
    .await
    .unwrap();
}
