use {
    crate::utils::{self, pda},
    adrena::state::position::Position,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn cancel_take_profit(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    position_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;
    let custody_pda = position_account.custody;

    // Prepare PDA and addresses
    let cortex_pda = pda::get_cortex_pda().0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::CancelTakeProfit {
            owner: owner.pubkey(),
            cortex: cortex_pda,
            pool: position_account.pool,
            position: *position_pda,
            custody: custody_pda,
        }
        .to_account_metas(None),
        adrena::instruction::CancelTakeProfit {},
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("cancel_take_profit", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    let position_account_after =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    assert!(!position_account_after.take_profit_is_set());
    assert_eq!(position_account_after.take_profit_limit_price, 0);

    Ok(())
}
