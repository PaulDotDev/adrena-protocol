use {
    crate::utils::{self, pda},
    adrena::{self},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_limit_order_book(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let limit_order_book_pda = pda::get_limit_order_book_pda(pool_pda, &owner.pubkey()).0;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitLimitOrderBook {
            owner: owner.pubkey(),
            pool: *pool_pda,
            system_program: anchor_lang::system_program::ID,
            limit_order_book: limit_order_book_pda,
        }
        .to_account_metas(None),
        adrena::instruction::InitLimitOrderBook {},
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================

    Ok(())
}
