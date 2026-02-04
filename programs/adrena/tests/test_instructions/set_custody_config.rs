use {
    crate::utils::{self, pda},
    adrena::{instructions::SetCustodyConfigParams, state::custody::Custody},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_custody_config(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_pda: &Pubkey,
    params: SetCustodyConfigParams,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetCustodyConfig {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: *custody_pda,
        }
        .to_account_metas(None),
        adrena::instruction::SetCustodyConfig {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_custody_config", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, *custody_pda).await;

    // Check custody account
    {
        assert_eq!(custody_account.pool, *pool_pda);
        assert_eq!(custody_account.is_stable, params.is_stable as u8);
        assert_eq!(custody_account.oracle, params.oracle);
        assert_eq!(custody_account.pricing, params.pricing);
        assert_eq!(custody_account.fees, params.fees);
    }

    Ok(())
}
