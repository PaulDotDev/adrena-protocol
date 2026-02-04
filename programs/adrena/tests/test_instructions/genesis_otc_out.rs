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

#[allow(clippy::too_many_arguments)]
pub async fn genesis_otc_out(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    dao_receiving_account: &Pubkey,
    pool_pda: &Pubkey,
    usdc_mint: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;

    let custody_usdc_pda = pda::get_custody_pda(pool_pda, usdc_mint).0;
    let custody_usdc_token_account_pda = pda::get_custody_token_account_pda(pool_pda, usdc_mint).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::GenesisOtcOut {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            genesis_lock: genesis_lock_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            dao_receiving_account: *dao_receiving_account,
            transfer_authority: transfer_authority_pda,
            custody_usdc: custody_usdc_pda,
            custody_usdc_token_account: custody_usdc_token_account_pda,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::GenesisOtcOut {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("genesis_otc_out", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    Ok(())
}
