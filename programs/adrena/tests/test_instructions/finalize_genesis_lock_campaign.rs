use {
    crate::utils::{self, pda},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn finalize_genesis_lock_campaign(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::FinalizeGenesisLockCampaign {
            caller: payer.pubkey(),
            pool: *pool_pda,
            genesis_lock: genesis_lock_pda,
            cortex: cortex_pda,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::FinalizeGenesisLockCampaign {},
        Some(&payer.pubkey()),
        &[payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction(
        "finalize_genesis_lock_campaign",
        "admin",
        ix_size,
        tx_used_cu,
    );

    // ==== THEN ==============================================================

    Ok(())
}
