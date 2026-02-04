use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{instructions::GetSwapAmountAndFeesParams, state::cortex::SwapAmountAndFees},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_swap_amount_and_fee(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    // Mint received by the User
    dispensing_custody_token_mint: &Pubkey,
    // Mint sent by the User
    receiving_custody_token_mint: &Pubkey,
    params: GetSwapAmountAndFeesParams,
) -> std::result::Result<SwapAmountAndFees, BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let dispensing_custody_pda = pda::get_custody_pda(pool_pda, dispensing_custody_token_mint).0;
    let receiving_custody_pda = pda::get_custody_pda(pool_pda, receiving_custody_token_mint).0;

    // ==== WHEN ==============================================================
    let result: SwapAmountAndFees = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetSwapAmountAndFees {
            cortex: cortex_pda,
            pool: *pool_pda,
            receiving_custody: receiving_custody_pda,
            dispensing_custody: dispensing_custody_pda,
            oracle: oracle_pda,
        }
        .to_account_metas(None),
        adrena::instruction::GetSwapAmountAndFees {
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
