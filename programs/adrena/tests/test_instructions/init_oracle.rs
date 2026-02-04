use {
    crate::utils::{self, pda},
    adrena::instructions::InitOracleParams,
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_oracle(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    params: &InitOracleParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitOracle {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            cortex: cortex_pda,
            oracle: oracle_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitOracle {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_oracle", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}
