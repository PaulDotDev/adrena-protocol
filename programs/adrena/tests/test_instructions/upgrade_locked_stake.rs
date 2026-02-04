use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter,
        instructions::UpgradeLockedStakeParams,
        math,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingType},
            user_staking::UserStaking,
        },
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn upgrade_locked_stake(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    params: UpgradeLockedStakeParams,
    staked_token_mint: &Pubkey,
    governance_realm_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(&owner.pubkey(), &staking_pda).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let staking_staked_token_vault_pda = pda::get_staking_staked_token_vault_pda(&staking_pda).0;
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

    let staking_account = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;
    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), staked_token_mint).0;

    let governance_governing_token_holding_pda = pda::get_governance_governing_token_holding_pda(
        governance_realm_pda,
        &governance_token_mint_pda,
    );

    let governance_realm_config_pda = pda::get_governance_realm_config_pda(governance_realm_pda);

    let governance_governing_token_owner_record_pda =
        pda::get_governance_governing_token_owner_record_pda(
            governance_realm_pda,
            &governance_token_mint_pda,
            &owner.pubkey(),
        );

    // // ==== WHEN ==============================================================
    // save account state before tx execution
    let staking_account_before = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;

    let user_staking_account_before =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let locked_stake_account_before = user_staking_account_before
        .locked_stakes
        .iter()
        .find(|ls| ls.id == params.locked_stake_id)
        .unwrap();

    let governance_governing_token_holding_balance_before =
        utils::get_token_account_balance(program_test_ctx, governance_governing_token_holding_pda)
            .await;

    let funding_account_before =
        utils::get_token_account_balance(program_test_ctx, funding_account_address).await;

    let reward_token_account_address = utils::find_associated_token_account(
        &owner.pubkey(),
        &cortex_account.fee_redistribution_mint,
    )
    .0;

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lm_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lm_token_mint_pda).0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;
    let staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&staking_pda).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::UpgradeLockedStake {
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            reward_token_account: reward_token_account_address,
            lm_token_account: lm_token_account_address,
            staking_staked_token_vault: staking_staked_token_vault_pda,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            transfer_authority: transfer_authority_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            cortex: cortex_pda,
            governance_token_mint: governance_token_mint_pda,
            pool: *pool_pda,
            genesis_lock: genesis_lock_pda,
            lm_token_mint: lm_token_mint_pda,
            fee_redistribution_mint: cortex_account.fee_redistribution_mint,
            governance_realm: *governance_realm_pda,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            adrena_program: adrena::ID,
            governance_program: spl_governance_program_adapter::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::UpgradeLockedStake { params },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("upgrade_locked_stake", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let staking_account_after = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;

    let governance_governing_token_holding_balance_after =
        utils::get_token_account_balance(program_test_ctx, governance_governing_token_holding_pda)
            .await;

    let user_staking_account_after =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let funding_account_after =
        utils::get_token_account_balance(program_test_ctx, funding_account_address).await;

    let locked_stake_account_after = user_staking_account_after
        .locked_stakes
        .iter()
        .find(|ls| ls.id == params.locked_stake_id)
        .unwrap();

    if let Some(additional_amount) = params.amount {
        // Check changes in staking account if params.amount is not None
        {
            assert_eq!(
                staking_account_after.nb_locked_tokens - staking_account_before.nb_locked_tokens,
                additional_amount
            );

            // Check staked token ATA balance
            assert_eq!(
                funding_account_before - additional_amount,
                funding_account_after,
            );
        }

        // <*> FIRST checks amount changed - duration not changed
        if params.locked_days.is_none() {
            // Depending on the lock duration, vote multiplier will differ
            let staking_option = UserStaking::get_locked_staking_option(
                (locked_stake_account_after.lock_duration / (3600 * 24)) as u32,
                staking_account.get_staking_type(),
            )
            .unwrap();

            // Check that the duration has not changed before and after
            {
                assert_eq!(
                    locked_stake_account_after.lock_duration,
                    locked_stake_account_before.lock_duration
                );
            }

            // Check voting power (only for LM staking type)
            {
                if staking_account.get_staking_type() == StakingType::LM {
                    let additional_voting_power = math::checked_as_u64(
                        (additional_amount * staking_option.vote_multiplier as u64) as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    assert_eq!(
                        governance_governing_token_holding_balance_before + additional_voting_power,
                        governance_governing_token_holding_balance_after,
                    );
                }
            }

            // Check rewards multipliers
            {
                let additional_rewards = math::checked_as_u64(
                    (additional_amount as u128 * staking_option.reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();
                let additional_lm_rewards = math::checked_as_u64(
                    (additional_amount as u128 * staking_option.lm_reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();

                // Check the amount with reward multipliers increased
                assert_eq!(
                    locked_stake_account_after.amount_with_lm_reward_multiplier
                        - additional_lm_rewards,
                    locked_stake_account_before.amount_with_lm_reward_multiplier
                );
                assert_eq!(
                    locked_stake_account_after.amount_with_reward_multiplier - additional_rewards,
                    locked_stake_account_before.amount_with_reward_multiplier
                );

                // Checks that actual rewards multipliers didn't change
                assert_eq!(
                    locked_stake_account_after.reward_multiplier,
                    locked_stake_account_before.reward_multiplier
                );
                assert_eq!(
                    locked_stake_account_after.lm_reward_multiplier,
                    locked_stake_account_before.lm_reward_multiplier
                );
            }
        }

        // <*> SECOND checks amount changed - duration changed
        if let Some(new_locked_days) = params.locked_days {
            let new_staking_option = UserStaking::get_locked_staking_option(
                (locked_stake_account_after.lock_duration / (3600 * 24)) as u32,
                staking_account.get_staking_type(),
            )
            .unwrap();

            // Check that the duration and amount locked have changed
            {
                assert_ne!(
                    locked_stake_account_after.lock_duration,
                    locked_stake_account_before.lock_duration
                );
                assert_ne!(
                    locked_stake_account_after.amount,
                    locked_stake_account_before.amount
                );

                // Check that new duration is new_locked_days * 24 * 3600
                assert_eq!(
                    locked_stake_account_after.lock_duration,
                    (new_locked_days as u64 * 24 * 3600)
                );
            }

            // Check voting power
            {
                if staking_account.get_staking_type() == StakingType::LM {
                    // Determine extra voting power from lock duration change on previously existing amount
                    let old_voting_power_on_staked_amount = math::checked_as_u64(
                        (locked_stake_account_before.amount
                            * locked_stake_account_before.vote_multiplier as u64)
                            as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    let new_voting_power_on_staked_amount = math::checked_as_u64(
                        (locked_stake_account_before.amount
                            * new_staking_option.vote_multiplier as u64)
                            as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    let additional_voting_power_from_duration_change =
                        new_voting_power_on_staked_amount - old_voting_power_on_staked_amount;

                    // Determine extra voting power from newly added amount (+ new lock duration)
                    let additional_voting_power_on_amount_change = math::checked_as_u64(
                        (additional_amount * new_staking_option.vote_multiplier as u64) as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    assert_eq!(
                        governance_governing_token_holding_balance_before
                            + additional_voting_power_on_amount_change
                            + additional_voting_power_from_duration_change,
                        governance_governing_token_holding_balance_after,
                    );
                }
            }

            // Check rewards multipliers
            {
                let mew_amount = locked_stake_account_before.amount + additional_amount;
                let new_rewards = math::checked_as_u64(
                    (mew_amount as u128 * new_staking_option.reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();
                let new_lm_rewards = math::checked_as_u64(
                    (mew_amount as u128 * new_staking_option.lm_reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();

                // Verify new amount with reward multipliers
                assert_eq!(
                    locked_stake_account_after.amount_with_lm_reward_multiplier,
                    new_lm_rewards
                );
                assert_eq!(
                    locked_stake_account_after.amount_with_reward_multiplier,
                    new_rewards
                );

                // Verify actual rewards multipliers
                assert_eq!(
                    locked_stake_account_after.reward_multiplier,
                    new_staking_option.reward_multiplier
                );
                assert_eq!(
                    locked_stake_account_after.lm_reward_multiplier,
                    new_staking_option.lm_reward_multiplier
                );

                // Verify that rewards multipliers did actually change between before and after
                assert_ne!(
                    locked_stake_account_after.reward_multiplier,
                    locked_stake_account_before.reward_multiplier
                );
                assert_ne!(
                    locked_stake_account_after.lm_reward_multiplier,
                    locked_stake_account_before.lm_reward_multiplier
                );
            }
        }
    }

    // <*> THIRD checks amount not changed - duration changed
    if let Some(new_locked_days) = params.locked_days {
        if params.amount.is_none() {
            let new_staking_option = UserStaking::get_locked_staking_option(
                new_locked_days,
                staking_account.get_staking_type(),
            )
            .unwrap();

            // Check that the amount locked has not changed before and after
            {
                assert_eq!(
                    locked_stake_account_after.amount,
                    locked_stake_account_before.amount
                );
            }

            // Check changes in staking account if params.amount is None
            {
                assert_eq!(
                    staking_account_after.nb_locked_tokens,
                    staking_account_before.nb_locked_tokens
                );

                // Check staked token ATA balance
                assert_eq!(funding_account_before, funding_account_after,);
            }

            // Check voting power
            {
                if staking_account.get_staking_type() == StakingType::LM {
                    // Determine extra voting power from lock duration change on previously existing amount
                    let old_voting_power_on_staked_amount = math::checked_as_u64(
                        (locked_stake_account_before.amount
                            * locked_stake_account_before.vote_multiplier as u64)
                            as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    let new_voting_power_on_staked_amount = math::checked_as_u64(
                        (locked_stake_account_before.amount
                            * new_staking_option.vote_multiplier as u64)
                            as u128
                            / Cortex::BPS_POWER,
                    )
                    .unwrap();

                    let additional_voting_power_on_previously_staked_amount =
                        new_voting_power_on_staked_amount - old_voting_power_on_staked_amount;

                    assert_eq!(
                        governance_governing_token_holding_balance_before
                            + additional_voting_power_on_previously_staked_amount,
                        governance_governing_token_holding_balance_after,
                    );
                }
            }

            // Check rewards multipliers
            {
                // before or after, same. But taking before for clarity (checked above anyway)
                let locked_amount = locked_stake_account_before.amount;

                let new_rewards = math::checked_as_u64(
                    (locked_amount as u128 * new_staking_option.reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();
                let new_lm_rewards = math::checked_as_u64(
                    (locked_amount as u128 * new_staking_option.lm_reward_multiplier as u128)
                        / Cortex::BPS_POWER,
                )
                .unwrap();

                assert_eq!(
                    locked_stake_account_after.amount_with_lm_reward_multiplier,
                    new_lm_rewards
                );
                assert_eq!(
                    locked_stake_account_after.amount_with_reward_multiplier,
                    new_rewards
                );

                assert_eq!(
                    locked_stake_account_after.reward_multiplier,
                    new_staking_option.reward_multiplier
                );
                assert_eq!(
                    locked_stake_account_after.lm_reward_multiplier,
                    new_staking_option.lm_reward_multiplier
                );
            }
        }
    }

    Ok(())
}
