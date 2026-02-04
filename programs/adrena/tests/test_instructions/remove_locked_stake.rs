use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter, instructions::RemoveLockedStakeParams,
        state::user_staking::UserStaking,
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn remove_locked_stake(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    params: RemoveLockedStakeParams,
    fee_redistribution_mint: &Pubkey,
    staked_token_mint: &Pubkey,
    pool_pda: &Pubkey,
    governance_realm_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(&owner.pubkey(), &staking_pda).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;
    let staking_staked_token_vault_pda = pda::get_staking_staked_token_vault_pda(&staking_pda).0;
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&staking_pda).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

    let lm_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lm_token_mint_pda).0;
    let staked_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), staked_token_mint).0;
    let staking_reward_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), fee_redistribution_mint).0;

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
    let user_staking_account_before =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let owner_staked_token_account_before =
        utils::get_token_account_balance(program_test_ctx, staked_token_account_address).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::RemoveLockedStake {
            owner: owner.pubkey(),
            lm_token_account: lm_token_account_address,
            staked_token_account: staked_token_account_address,
            reward_token_account: staking_reward_token_account_address,
            staking_staked_token_vault: staking_staked_token_vault_pda,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            transfer_authority: transfer_authority_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            genesis_lock: genesis_lock_pda,
            lm_token_mint: lm_token_mint_pda,
            staked_token_mint: *staked_token_mint,
            fee_redistribution_mint: *fee_redistribution_mint,
            governance_realm: *governance_realm_pda,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            governance_program: spl_governance_program_adapter::ID,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            governance_token_mint: governance_token_mint_pda,
        }
        .to_account_metas(None),
        adrena::instruction::RemoveLockedStake {
            params: RemoveLockedStakeParams {
                locked_stake_index: params.locked_stake_index,
            },
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("remove_locked_stake", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let owner_staked_token_account_after =
        utils::get_token_account_balance(program_test_ctx, staked_token_account_address).await;

    // Check owner staked token ATA balance
    {
        assert!(
            owner_staked_token_account_before
                + user_staking_account_before.locked_stakes[params.locked_stake_index as usize]
                    .amount
                - user_staking_account_before.locked_stakes[params.locked_stake_index as usize]
                    .early_exit_fee
                <= owner_staked_token_account_after,
        );
    }

    Ok(())
}
