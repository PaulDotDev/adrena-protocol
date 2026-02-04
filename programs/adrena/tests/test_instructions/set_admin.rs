use {
    crate::utils::{self, pda},
    adrena::{instructions::SetAdminParams, state::cortex::Cortex},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_admin(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    admin: &Keypair,
    new_admin: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetAdmin {
            admin: admin.pubkey(),
            cortex: cortex_pda,
        }
        .to_account_metas(None),
        adrena::instruction::SetAdmin {
            params: SetAdminParams {
                new_admin: *new_admin,
            },
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_admin", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    assert_eq!(cortex_account.admin, *new_admin);

    Ok(())
}
