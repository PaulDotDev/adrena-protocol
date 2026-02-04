use {
    crate::utils::{self, pda},
    adrena::state::cortex::Cortex,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_protocol_fee_recipient(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    new_protocol_fee_recipient: &Pubkey,
    fee_redistribution_mint: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetProtocolFeeRecipient {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            protocol_fee_recipient: *new_protocol_fee_recipient,
            fee_redistribution_mint: *fee_redistribution_mint,
        }
        .to_account_metas(None),
        adrena::instruction::SetProtocolFeeRecipient {},
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_protocol_fee_recipient", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    assert_eq!(
        cortex_account.protocol_fee_recipient,
        *new_protocol_fee_recipient
    );

    Ok(())
}
