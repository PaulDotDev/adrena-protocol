use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{instructions::GetRemoveLiquidityAmountAndFeeParams, state::cortex::AmountAndFee},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

pub async fn get_remove_liquidity_amount_and_fee(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    params: GetRemoveLiquidityAmountAndFeeParams,
) -> std::result::Result<AmountAndFee, BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;

    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;

    let result: AmountAndFee = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetRemoveLiquidityAmountAndFee {
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            lp_token_mint: lp_token_mint_pda,
        }
        .to_account_metas(None),
        adrena::instruction::GetRemoveLiquidityAmountAndFee {
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
