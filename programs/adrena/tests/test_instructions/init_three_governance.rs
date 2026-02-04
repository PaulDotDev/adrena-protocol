use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter,
        state::cortex::{Cortex, CortexInitializationStep},
    },
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn init_three_governance(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    governance_realm_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let (transfer_authority_pda, _) = pda::get_transfer_authority_pda();
    let (cortex_pda, _) = pda::get_cortex_pda();
    let (governance_token_mint_pda, _) = pda::get_governance_token_mint_pda();

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitThreeGovernance {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            governance_token_mint: governance_token_mint_pda,
            governance_realm: *governance_realm_pda,
            governance_program: spl_governance_program_adapter::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitThreeGovernance {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_three_governance", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    // Assert cortex
    {
        assert_eq!(
            cortex_account.get_initialized(),
            CortexInitializationStep::Step3
        );
    }

    Ok(())
}
