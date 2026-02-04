use {
    crate::utils::{self, pda},
    adrena::{
        self,
        instructions::AddLimitOrderParams,
        state::{limit_order_book::LimitOrderBook, pool::Pool},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn add_limit_order(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    collateral_custody_token_mint: &Pubkey,
    params: AddLimitOrderParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let custody_pda: Pubkey = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;
    let cortex_pda = pda::get_cortex_pda().0;

    let limit_order_book_pda = pda::get_limit_order_book_pda(pool_pda, &owner.pubkey()).0;
    let collateral_escrow_pda =
        pda::get_collateral_escrow_pda(pool_pda, &owner.pubkey(), collateral_custody_token_mint).0;
    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), collateral_custody_token_mint).0;

    let limit_order_book_account_before =
        utils::try_get_zero_copy_account::<LimitOrderBook>(program_test_ctx, limit_order_book_pda)
            .await;
    let pool_account_before = utils::get_account::<Pool>(program_test_ctx, *pool_pda).await;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddLimitOrder {
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: custody_pda,
            collateral_custody_mint: *collateral_custody_token_mint,
            collateral_custody: collateral_custody_pda,
            system_program: anchor_lang::system_program::ID,
            limit_order_book: limit_order_book_pda,
            collateral_escrow: collateral_escrow_pda,
            associated_token_program: spl_associated_token_account::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddLimitOrder { params },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================
    let limit_order_book_account_after =
        utils::get_account::<LimitOrderBook>(program_test_ctx, limit_order_book_pda).await;
    let pool_account_after = utils::get_account::<Pool>(program_test_ctx, *pool_pda).await;

    assert_eq!(
        limit_order_book_account_after.registered_limit_order_count,
        if let Some(b) = limit_order_book_account_before {
            b.registered_limit_order_count + 1
        } else {
            1
        }
    );

    assert_eq!(
        pool_account_after.unique_limit_order_id_counter,
        if pool_account_before.unique_limit_order_id_counter == 0 {
            2 // 0 isn't used, so 1 is the first and 2 is the second
        } else {
            pool_account_before.unique_limit_order_id_counter + 1
        }
    );

    Ok(())
}
