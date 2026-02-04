use {
    crate::utils::{self, pda},
    adrena::instructions::GenesisOtcInParams,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn genesis_otc_in(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_one_token_mint: &Pubkey,
    custody_two_token_mint: &Pubkey,
    custody_three_token_mint: &Pubkey,
    params: GenesisOtcInParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;

    let custody_one_pda = pda::get_custody_pda(pool_pda, custody_one_token_mint).0;
    let custody_one_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_one_token_mint).0;

    let custody_two_pda = pda::get_custody_pda(pool_pda, custody_two_token_mint).0;
    let custody_two_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_two_token_mint).0;

    let custody_three_pda = pda::get_custody_pda(pool_pda, custody_three_token_mint).0;
    let custody_three_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_three_token_mint).0;

    let funding_account_one_address =
        utils::find_associated_token_account(&admin.pubkey(), custody_one_token_mint).0;
    let funding_account_two_address =
        utils::find_associated_token_account(&admin.pubkey(), custody_two_token_mint).0;
    let funding_account_three_address =
        utils::find_associated_token_account(&admin.pubkey(), custody_three_token_mint).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::GenesisOtcIn {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            funding_account_one: funding_account_one_address,
            funding_account_two: funding_account_two_address,
            funding_account_three: funding_account_three_address,
            custody_one: custody_one_pda,
            custody_one_token_account: custody_one_token_account_pda,
            custody_two: custody_two_pda,
            custody_two_token_account: custody_two_token_account_pda,
            custody_three: custody_three_pda,
            custody_three_token_account: custody_three_token_account_pda,
            genesis_lock: genesis_lock_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::GenesisOtcIn { params },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("genesis_otc_in", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    Ok(())
}
