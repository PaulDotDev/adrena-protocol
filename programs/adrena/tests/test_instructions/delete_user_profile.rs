use {
    crate::utils::{self, pda},
    adrena::state::user_profile::UserProfile,
    anchor_lang::ToAccountMetas,
    solana_program::pubkey::Pubkey,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn delete_user_profile(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    user: &Pubkey,
    payer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let cortex_pda: Pubkey = pda::get_cortex_pda().0;
    let user_profile_pda = pda::get_user_profile_pda(user).0;

    // ==== WHEN ==============================================================

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::DeleteUserProfile {
            admin: admin.pubkey(),
            user: *user,
            payer: payer.pubkey(),
            user_profile: user_profile_pda,
            cortex: cortex_pda,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        adrena::instruction::DeleteUserProfile {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("delete_user_profile", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let user_profile_account =
        utils::try_get_zero_copy_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    assert!(user_profile_account.is_none());

    Ok(())
}
