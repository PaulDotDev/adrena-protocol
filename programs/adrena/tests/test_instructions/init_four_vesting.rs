use {
    crate::utils::{self, pda},
    adrena::state::cortex::{Cortex, CortexInitializationStep},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_four_vesting(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let (transfer_authority_pda, _) = pda::get_transfer_authority_pda();
    let (cortex_pda, _) = pda::get_cortex_pda();
    let (vest_registry_pda, _) = pda::get_vest_registry_pda();

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitFourVesting {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            vest_registry: vest_registry_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitFourVesting {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_four_vesting", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    // Assert cortex
    {
        assert_eq!(
            cortex_account.get_initialized(),
            CortexInitializationStep::Initialized
        );
    }

    Ok(())
}
