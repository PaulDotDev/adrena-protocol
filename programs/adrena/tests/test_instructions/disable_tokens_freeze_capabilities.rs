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

pub async fn disable_tokens_freeze_capabilities(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::DisableTokensFreezeCapabilities {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
            transfer_authority: transfer_authority_pda,
            lm_token_mint: lm_token_mint_pda,
            lp_token_mint: lp_token_mint_pda,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::DisableTokensFreezeCapabilities {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction(
        "disable_tokens_freeze_capabilities",
        "admin",
        ix_size,
        tx_used_cu,
    );

    // ==== THEN ==============================================================
    Ok(())
}
