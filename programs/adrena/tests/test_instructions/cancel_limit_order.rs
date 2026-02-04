use {
    crate::utils::{self, pda},
    adrena::{self, instructions::CancelLimitOrderParams, state::limit_order_book::LimitOrderBook},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn cancel_limit_order(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    collateral_custody_token_mint: &Pubkey,
    params: CancelLimitOrderParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;
    let cortex_pda = pda::get_cortex_pda().0;

    let limit_order_book_pda = pda::get_limit_order_book_pda(pool_pda, &owner.pubkey()).0;
    let collateral_escrow_pda =
        pda::get_collateral_escrow_pda(pool_pda, &owner.pubkey(), collateral_custody_token_mint).0;
    let receiving_account_address =
        utils::find_associated_token_account(&owner.pubkey(), collateral_custody_token_mint).0;

    let limit_order_book_account_before =
        utils::get_account::<LimitOrderBook>(program_test_ctx, limit_order_book_pda).await;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::CancelLimitOrder {
            owner: owner.pubkey(),
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            collateral_custody_mint: *collateral_custody_token_mint,
            collateral_custody: collateral_custody_pda,
            system_program: anchor_lang::system_program::ID,
            limit_order_book: limit_order_book_pda,
            collateral_escrow: collateral_escrow_pda,
            associated_token_program: spl_associated_token_account::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::CancelLimitOrder { params },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================

    let limit_order_book_account_after =
        utils::try_get_zero_copy_account::<LimitOrderBook>(program_test_ctx, limit_order_book_pda)
            .await;

    assert_eq!(
        limit_order_book_account_before.registered_limit_order_count - 1,
        if let Some(b) = limit_order_book_account_after {
            b.registered_limit_order_count
        } else {
            0
        },
    );

    Ok(())
}
