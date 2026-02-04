use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{instructions::GetEntryPriceAndFeeParams, state::cortex::NewPositionPricesAndFee},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_entry_price_and_fee(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    collateral_custody_token_mint: &Pubkey,
    params: GetEntryPriceAndFeeParams,
) -> std::result::Result<NewPositionPricesAndFee, BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;

    let result: NewPositionPricesAndFee = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetEntryPriceAndFee {
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: custody_pda,
            collateral_custody: collateral_custody_pda,
            oracle: oracle_pda,
        }
        .to_account_metas(None),
        adrena::instruction::GetEntryPriceAndFee {
            params: params.clone(),
        },
        payer,
        Some(get_update_pool_ix(program_test_ctx, payer, pool_pda, params.oracle_prices).await?),
        None,
    )
    .await?;

    // ==== THEN ==============================================================
    Ok(result)
}
