use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLockedStakeParams, AddVestParams, BucketName, ClaimStakesParams,
            UpgradeLockedStakeParams,
        },
        state::{
            cortex::Cortex,
            staking::{Staking, StakingRound},
        },
        utils::limited_string::LimitedString,
    },
    anchor_spl::token::spl_token,
    maplit::hashmap,
    solana_sdk::{signer::Signer, transaction::Transaction},
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn patch_staking_round() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000000, USDC_DECIMALS),
                    "eth" => utils::scale(50, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(50, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(50, ETH_DECIMALS),
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
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        utils::scale(100_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");
    let martin = test_setup.get_user_keypair_by_name("martin");
    let paul = test_setup.get_user_keypair_by_name("paul");

    let usdc_mint = test_setup.get_mint_by_name("usdc");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    {
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();

        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();

        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            paul,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
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
                amount: utils::scale(100, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            martin,
            &AddVestParams {
                amount: utils::scale(100, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            paul,
            &AddVestParams {
                amount: utils::scale(100, Cortex::LM_DECIMALS),
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

        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();

        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            paul,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    {
        {
            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            test_instructions::add_locked_stake(
                &test_setup.program_test_ctx,
                alice,
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
        }

        {
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
        }

        {
            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            test_instructions::add_locked_stake(
                &test_setup.program_test_ctx,
                paul,
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
        }
    }

    // Generate 32 Staking Rounds (1st no reward)
    for i in 0..33 {
        {
            // 100 USDC of REWARDS
            {
                let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;

                let lm_staking_reward_token_vault_pda =
                    pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;

                let alice_usdc_ata =
                    utils::find_associated_token_account(&alice.pubkey(), &usdc_mint).0;

                let transfer_ix = spl_token::instruction::transfer(
                    &spl_token::id(),
                    &alice_usdc_ata,
                    &lm_staking_reward_token_vault_pda,
                    &alice.pubkey(),
                    &[],
                    100000000,
                )
                .unwrap();

                let mut ctx = test_setup.program_test_ctx.write().await;
                let last_blockhash = ctx.last_blockhash;
                let banks_client = &mut ctx.banks_client;

                let tx = Transaction::new_signed_with_payer(
                    &[transfer_ix],                           // The transfer instruction
                    Some(&test_setup.payer_keypair.pubkey()), // Fee payer
                    &[&test_setup.payer_keypair, &alice], // Signers (payer and authority of the source account)
                    last_blockhash,
                );

                // Process the transaction
                banks_client.process_transaction(tx).await.unwrap();
            }
        }

        // Resolve current round
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

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;
        }

        // Claim first round with alice so she can upgrade the locked stake
        // The bug will appear resolved_staking_round[1]
        if i == 1 {
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
        }

        if i == 5 {
            test_instructions::claim_stakes(
                &test_setup.program_test_ctx,
                martin,
                &test_setup.payer_keypair,
                &martin.pubkey(),
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
                paul,
                &test_setup.payer_keypair,
                &paul.pubkey(),
                &test_setup.pool_pda,
                &lm_token_mint_pda,
                &ClaimStakesParams {
                    locked_stake_indexes: None,
                },
            )
            .await
            .unwrap();
        }
    }

    // Create the BUG
    {
        test_instructions::upgrade_locked_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &test_setup.pool_pda,
            UpgradeLockedStakeParams {
                amount: Some(utils::scale(2, Cortex::LM_DECIMALS)),
                locked_days: None,
                locked_stake_id: 0,
            },
            &test_setup.lm_token_mint,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

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

    //
    //
    // HERE THE BUG EXISTS
    //
    //

    {
        let funding_account_address =
            utils::find_associated_token_account(&test_setup.admin_keypair.pubkey(), &usdc_mint).0;

        // Fund the admin wallet
        {
            let alice_usdc_ata =
                utils::find_associated_token_account(&alice.pubkey(), &usdc_mint).0;

            let transfer_ix = spl_token::instruction::transfer(
                &spl_token::id(),
                &alice_usdc_ata,
                &funding_account_address,
                &alice.pubkey(),
                &[],
                utils::scale(5000, USDC_DECIMALS),
            )
            .unwrap();

            let mut ctx = test_setup.program_test_ctx.write().await;
            let last_blockhash = ctx.last_blockhash;
            let banks_client = &mut ctx.banks_client;

            let tx = Transaction::new_signed_with_payer(
                &[transfer_ix],                           // The transfer instruction
                Some(&test_setup.payer_keypair.pubkey()), // Fee payer
                &[&test_setup.payer_keypair, &alice], // Signers (payer and authority of the source account)
                last_blockhash,
            );

            // Process the transaction
            banks_client.process_transaction(tx).await.unwrap();
        }
    }

    test_instructions::patch_staking_round(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        &usdc_mint,
        &lm_token_mint_pda,
    )
    .await
    .unwrap();

    //
    //
    // HERE THE BUG IS PATCHED
    //
    //

    test_instructions::claim_stakes(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &martin.pubkey(),
        &test_setup.pool_pda,
        &lm_token_mint_pda,
        &ClaimStakesParams {
            locked_stake_indexes: None,
        },
    )
    .await
    .unwrap();

    //
    // Check that the bug is gone
    //

    // Generate 32 Staking Rounds
    for _ in 0..33 {
        // Resolve current round
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

        utils::warp_forward(&test_setup.program_test_ctx, 1).await;

        {
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

            utils::warp_forward(&test_setup.program_test_ctx, 1).await;

            test_instructions::claim_stakes(
                &test_setup.program_test_ctx,
                martin,
                &test_setup.payer_keypair,
                &martin.pubkey(),
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
                paul,
                &test_setup.payer_keypair,
                &paul.pubkey(),
                &test_setup.pool_pda,
                &lm_token_mint_pda,
                &ClaimStakesParams {
                    locked_stake_indexes: None,
                },
            )
            .await
            .unwrap();
        }
    }

    //
    // LOOK WHAT IS IN THE STAKING AFTER THE PATCH
    //

    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;

    let lm_staking =
        utils::get_account::<Staking>(&test_setup.program_test_ctx, lm_staking_pda).await;

    assert_eq!(lm_staking.resolved_lm_reward_token_amount, 1904444444513); // 2M minus debt
    assert_eq!(lm_staking.resolved_lm_staked_token_amount, 49999986000000); // 50M minus debt

    assert_eq!(lm_staking.resolved_reward_token_amount, 4866666701); // 5k minus debt
    assert_eq!(lm_staking.resolved_staked_token_amount, 49999980000000); // 50M minus debt

    let lm_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;
    let lm_staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&lm_staking_pda).0;

    let lm_staking_reward_token_vault_balance = utils::get_token_account_balance(
        &test_setup.program_test_ctx,
        lm_staking_reward_token_vault_pda,
    )
    .await;

    let lm_staking_lm_reward_token_vault_balance = utils::get_token_account_balance(
        &test_setup.program_test_ctx,
        lm_staking_lm_reward_token_vault_pda,
    )
    .await;

    assert_eq!(lm_staking_reward_token_vault_balance, 4866666701);
    assert_eq!(lm_staking_lm_reward_token_vault_balance, 1904444444513);
}
