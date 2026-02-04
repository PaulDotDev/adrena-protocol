use {
    crate::utils::{self, pda},
    adrena::state::pool::Pool,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_pool_whitelisted_swapper(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    swapper: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetPoolWhitelistedSwapper {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
            whitelisted_swapper: *swapper,
        }
        .to_account_metas(None),
        adrena::instruction::SetPoolWhitelistedSwapper {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_pool_whitelisted_swapper", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, *pool_pda).await;

    assert_eq!(
        pool_account.whitelisted_swapper.to_string(),
        swapper.to_string()
    );

    Ok(())
}
