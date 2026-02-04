use {
    crate::utils::{self, pda},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn patch_staking_round(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    fee_redistribution_mint: &Pubkey,
    staked_mint: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let staking_pda = pda::get_staking_pda(staked_mint).0;

    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let staking_lm_reward_token_vault_pda =
        utils::get_staking_lm_reward_token_vault_pda(&staking_pda).0;

    let funding_account_address =
        utils::find_associated_token_account(&admin.pubkey(), fee_redistribution_mint).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::PatchStakingRound {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            fee_redistribution_mint: *fee_redistribution_mint,
            lm_token_mint: lm_token_mint_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            funding_account: funding_account_address,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            staking: staking_pda,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::PatchStakingRound {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("patch_staking_round", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    Ok(())
}
