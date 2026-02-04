use {
    crate::utils::{self, pda},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_staking_two(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    fee_redistribution_mint: &Pubkey,
    staked_token_mint: &Pubkey,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let (staking_pda, staking_bump) = pda::get_staking_pda(staked_token_mint);
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitStakingTwo {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            staking: staking_pda,
            lm_token_mint: lm_token_mint_pda,
            cortex: cortex_pda,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            fee_redistribution_mint: *fee_redistribution_mint,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitStakingTwo {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_staking_two", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    // TODO, check the created staking account

    Ok((staking_pda, staking_bump))
}
