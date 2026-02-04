use {
    crate::utils::{self, pda},
    adrena::{
        instructions::InitOneParams,
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

pub async fn init_one_core(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    protocol_fee_recipient: &Pubkey,
    fee_redistribution_mint: &Pubkey,
    params: InitOneParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let (lm_token_mint_pda, lm_token_mint_bump) = pda::get_lm_token_mint_pda();
    let (transfer_authority_pda, transfer_authority_bump) = pda::get_transfer_authority_pda();
    let (cortex_pda, cortex_bump) = pda::get_cortex_pda();

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitOne {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            protocol_fee_recipient: *protocol_fee_recipient,
            fee_redistribution_mint: *fee_redistribution_mint,
            lm_token_mint: lm_token_mint_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitOneCore { params },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_one_core", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    // Assert cortex
    {
        assert_eq!(cortex_account.admin, admin.pubkey());
        assert_eq!(
            cortex_account.get_initialized(),
            CortexInitializationStep::Step1
        );

        assert_eq!(cortex_account.lm_token_bump, lm_token_mint_bump);
        assert_eq!(cortex_account.bump, cortex_bump);
        assert_eq!(
            cortex_account.transfer_authority_bump,
            transfer_authority_bump
        );
    }

    Ok(())
}
