use {
    crate::utils::{self, pda},
    adrena::state::cortex::{Cortex, CortexInitializationStep},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_two_lm_token_metadata(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let (lm_token_mint_pda, _) = pda::get_lm_token_mint_pda();
    let (cortex_pda, _) = pda::get_cortex_pda();
    let (lm_token_mint_metadata_pda, _) = pda::get_metadata_pda(&lm_token_mint_pda);
    let (transfer_authority_pda, _) = pda::get_transfer_authority_pda();

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitTwoLmTokenMetadata {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            lm_token_mint: lm_token_mint_pda,
            lm_token_mint_metadata: lm_token_mint_metadata_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            mpl_token_metadata_program: mpl_token_metadata::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitTwoLmTokenMetadata {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_two_lm_token_metadata", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    // Assert cortex
    {
        assert_eq!(
            cortex_account.get_initialized(),
            CortexInitializationStep::Step2
        );
    }

    Ok(())
}
