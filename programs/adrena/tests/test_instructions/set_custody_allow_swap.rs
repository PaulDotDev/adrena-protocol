use {
    crate::utils::{self, pda},
    adrena::{instructions::SetCustodyAllowSwapParams, state::custody::Custody},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_custody_allow_swap(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_pda: &Pubkey,
    allow_swap: bool,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetCustodyAllowSwap {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: *custody_pda,
        }
        .to_account_metas(None),
        adrena::instruction::SetCustodyAllowSwap {
            params: SetCustodyAllowSwapParams { allow_swap },
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_custody_allow_swap", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, *custody_pda).await;

    assert_eq!(custody_account.allow_swap, allow_swap as u8);

    Ok(())
}
