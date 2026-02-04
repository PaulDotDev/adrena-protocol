use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{instructions::ExecuteLimitOrderShortParams, state::position::Side},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn execute_limit_order_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    collateral_custody_token_mint: &Pubkey,
    params: ExecuteLimitOrderShortParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;

    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;

    let (position_pda, position_bump) =
        pda::get_position_pda(&owner.pubkey(), pool_pda, &custody_pda, Side::Short);

    let cortex_pda = pda::get_cortex_pda().0;

    let collateral_custody_pda: Pubkey =
        pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;
    let collateral_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, collateral_custody_token_mint).0;

    let collateral_escrow_pda =
        pda::get_collateral_escrow_pda(pool_pda, &owner.pubkey(), collateral_custody_token_mint).0;
    let limit_order_book_pda = pda::get_limit_order_book_pda(pool_pda, &owner.pubkey()).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ExecuteLimitOrderShort {
            owner: owner.pubkey(),
            caller: caller.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            position: position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_escrow: collateral_escrow_pda,
            limit_order_book: limit_order_book_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
            collateral_custody: collateral_custody_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
        }
        .to_account_metas(None),
        adrena::instruction::ExecuteLimitOrderShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[caller, payer],
        Some(vec![
            get_update_pool_ix(program_test_ctx, payer, pool_pda, params.oracle_prices).await?,
        ]),
        None,
    )
    .await?;

    utils::log_instruction("execute_limit_order_short", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok((position_pda, position_bump))
}
