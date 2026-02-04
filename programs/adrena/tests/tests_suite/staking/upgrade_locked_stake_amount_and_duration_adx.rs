use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLiquidityParams, AddLockedStakeParams, AddVestParams, BucketName, ClaimStakesParams,
            RemoveLockedStakeParams, UpgradeLockedStakeParams,
        },
        state::{cortex::Cortex, staking::StakingRound},
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn upgrade_locked_stake_amount_and_duration_adx() {
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

    let alice_staking_reward_token_account_address = utils::find_associated_token_account(
        &alice.pubkey(),
        &test_setup.get_fee_redistribution_mint(),
    )
    .0;

    let alice_lm_token_account_address =
        utils::find_associated_token_account(&alice.pubkey(), &lm_token_mint_pda).0;

    // All the non focused on the test case setup is done here
    prep_work(&test_setup, alice, martin, &lm_token_mint_pda, eth_mint).await;

    // Upgrade to nothing (should fail)
    {
        assert!(test_instructions::upgrade_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            UpgradeLockedStakeParams {
                amount: None,
                locked_days: None,
                locked_stake_id: 0,
            },
            &test_setup.lm_token_mint,
            &test_setup.governance_realm_pda,
        )
        .await
        .is_err());
    }

    // Upgrade to same duration and same amount(+0) (should fail)
    {
        assert!(test_instructions::upgrade_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            UpgradeLockedStakeParams {
                amount: Some(utils::scale(0, Cortex::LM_DECIMALS)),
                locked_days: Some(180),
                locked_stake_id: 0,
            },
            &test_setup.lm_token_mint,
            &test_setup.governance_realm_pda,
        )
        .await
        .is_err());
    }

    // Upgrade to invalid duration and invalid amount(+0) (should fail)
    {
        assert!(test_instructions::upgrade_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            UpgradeLockedStakeParams {
                amount: Some(utils::scale(0, Cortex::LM_DECIMALS)),
                locked_days: Some(1),
                locked_stake_id: 0,
            },
            &test_setup.lm_token_mint,
            &test_setup.governance_realm_pda,
        )
        .await
        .is_err());
    }

    generate_rewards_for_current_round_warp_to_next_round_and_resolve(
        &test_setup,
        martin,
        alice,
        eth_mint,
    )
    .await;

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
            &test_setup.lm_token_mint,
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

        // 1 * 2.5 + 2 * 2.5 = 7.5          // 2.5 / 7 = 35% rewards USDC
        // 1 * 1.75 + 2 * 1.75 = 5.25       // 1.75 / 5.25 = 33% rewards ADX
        assert_eq!(balance_after - balance_before, 25000);
        assert_eq!(lm_balance_after - lm_balance_before, 23888888888);
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Upgrade the stake amount by 1 (totalling 2) and the duration by 2x
    {
        test_instructions::upgrade_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            UpgradeLockedStakeParams {
                amount: Some(utils::scale(1, Cortex::LM_DECIMALS)),
                locked_days: Some(540),
                locked_stake_id: 0,
            },
            &test_setup.lm_token_mint,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    generate_rewards_for_current_round_warp_to_next_round_and_resolve(
        &test_setup,
        martin,
        alice,
        eth_mint,
    )
    .await;

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
            &test_setup.lm_token_mint,
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

        // 2 * 4 + 2 * 2.5 = 13             // 8 / 13 = 61% rewards USDC
        // 2 * 3.25 + 2 * 1.75 = 10         // 6.5 / 10 = 65% rewards ADX
        assert_eq!(balance_after - balance_before, 46153);
        assert_eq!(lm_balance_after - lm_balance_before, 46583333332);
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    finalize_and_remove_stake(&test_setup, alice, &cortex_stake_reward_mint).await;
}

// Helpers ---------------------------------------------------------------------

async fn warp_to_next_round_and_resolve(test_setup: &utils::TestSetup, alice: &Keypair) {
    utils::warp_forward(
        &test_setup.program_test_ctx,
        StakingRound::ROUND_MIN_DURATION_SECONDS,
    )
    .await;

    test_instructions::resolve_staking_round(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.lm_token_mint,
    )
    .await
    .unwrap();
}

async fn generate_rewards_for_current_round_warp_to_next_round_and_resolve(
    test_setup: &utils::TestSetup,
    martin: &Keypair,
    alice: &Keypair,
    eth_mint: &Pubkey,
) {
    // Use add liquidity to generate rewards for the current round
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
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    // Warp to the next round and resolve the current one
    warp_to_next_round_and_resolve(test_setup, alice).await;
}

async fn finalize_and_remove_stake(
    test_setup: &utils::TestSetup,
    alice: &Keypair,
    cortex_stake_reward_mint: &Pubkey,
) {
    // Move 540d in the future where staking have ended
    utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(540)).await;

    test_instructions::resolve_staking_round(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &test_setup.lm_token_mint,
    )
    .await
    .unwrap();

    utils::execute_claim_stakes_automation(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &alice.pubkey(),
        &test_setup.pool_pda,
        &test_setup.lm_token_mint,
    )
    .await
    .unwrap();

    // Remove the stake without resolving it first should fail
    assert!(test_instructions::remove_locked_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        RemoveLockedStakeParams {
            locked_stake_index: 0,
        },
        cortex_stake_reward_mint,
        &test_setup.lm_token_mint,
        &test_setup.pool_pda,
        &test_setup.governance_realm_pda,
    )
    .await
    .is_err());

    // Trigger automation
    utils::execute_finalize_locked_stake_automation(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        &alice.pubkey(),
        &test_setup.governance_realm_pda,
        &test_setup.lm_token_mint,
        0,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;
}

async fn prep_work(
    test_setup: &utils::TestSetup,
    alice: &Keypair,
    martin: &Keypair,
    lm_token_mint_pda: &Pubkey,
    eth_mint: &Pubkey,
) {
    {
        // Martin: starts 180d locked staking
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &test_setup.lm_token_mint,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();

        // Alice: start 180d locked staking
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

    // Add vests for Alice and Martin
    for user in [alice, martin] {
        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            user,
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
    }

    // Move until vest end and claim vests
    utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

    for user in [alice, martin] {
        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            user,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    test_instructions::add_locked_stake(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        AddLockedStakeParams {
            amount: utils::scale(2, Cortex::LM_DECIMALS),
            locked_days: 180,
        },
        &test_setup.lm_token_mint,
        &test_setup.governance_realm_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    test_instructions::add_locked_stake(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        AddLockedStakeParams {
            amount: utils::scale(1, Cortex::LM_DECIMALS),
            locked_days: 180,
        },
        lm_token_mint_pda,
        &test_setup.governance_realm_pda,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let alice_staking_reward_token_account_address = utils::find_associated_token_account(
        &alice.pubkey(),
        &test_setup.get_fee_redistribution_mint(),
    )
    .0;

    let alice_lm_token_account_address =
        utils::find_associated_token_account(&alice.pubkey(), lm_token_mint_pda).0;

    // Claim when there is nothing to claim yet
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
            &test_setup.lm_token_mint,
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
    warp_to_next_round_and_resolve(test_setup, alice).await;

    generate_rewards_for_current_round_warp_to_next_round_and_resolve(
        test_setup, martin, alice, eth_mint,
    )
    .await;

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
            &test_setup.lm_token_mint,
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

        assert_eq!(balance_after - balance_before, 225000);
        assert_eq!(lm_balance_after - lm_balance_before, 47_777_777_777);
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;
}
