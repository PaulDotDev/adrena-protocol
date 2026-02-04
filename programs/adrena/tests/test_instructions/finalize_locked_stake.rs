use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter, instructions::FinalizeLockedStakeParams,
        state::user_staking::UserStaking,
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn finalize_locked_stake(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    owner: &Pubkey,
    payer: &Keypair,
    staked_token_mint: &Pubkey,
    governance_realm_pda: &Pubkey,
    locked_stake_index: usize,
    is_early_exit: bool,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

    let governance_governing_token_holding_pda = pda::get_governance_governing_token_holding_pda(
        governance_realm_pda,
        &governance_token_mint_pda,
    );

    let governance_realm_config_pda = pda::get_governance_realm_config_pda(governance_realm_pda);

    let governance_governing_token_owner_record_pda =
        pda::get_governance_governing_token_owner_record_pda(
            governance_realm_pda,
            &governance_token_mint_pda,
            owner,
        );

    // ==== WHEN ==============================================================
    // save account state before tx execution
    let user_staking_account =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let locked_stake_id = user_staking_account.locked_stakes[locked_stake_index].id;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::FinalizeLockedStake {
            caller: caller.pubkey(),
            owner: *owner,
            transfer_authority: transfer_authority_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            cortex: cortex_pda,
            lm_token_mint: lm_token_mint_pda,
            governance_token_mint: governance_token_mint_pda,
            governance_realm: *governance_realm_pda,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            governance_program: spl_governance_program_adapter::ID,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::FinalizeLockedStake {
            params: FinalizeLockedStakeParams {
                locked_stake_id,
                early_exit: is_early_exit,
            },
        },
        Some(&payer.pubkey()),
        &[caller, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("finalize_locked_stake", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}
