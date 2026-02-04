use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::GetOpenPositionWithSwapAmountAndFeesParams,
        state::cortex::OpenPositionWithSwapAmountAndFees,
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_open_position_with_swap_amount_and_fees(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    // Mint sent by the user
    receiving_custody_token_mint: &Pubkey,
    // Mint used as collateral
    collateral_custody_token_mint: &Pubkey,
    // Targeted principal
    principal_custody_token_mint: &Pubkey,
    params: GetOpenPositionWithSwapAmountAndFeesParams,
) -> std::result::Result<OpenPositionWithSwapAmountAndFees, BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let receiving_custody_pda = pda::get_custody_pda(pool_pda, receiving_custody_token_mint).0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;
    let principal_custody_pda = pda::get_custody_pda(pool_pda, principal_custody_token_mint).0;

    let result: OpenPositionWithSwapAmountAndFees = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetOpenPositionWithSwapAmountAndFees {
            cortex: cortex_pda,
            pool: *pool_pda,
            receiving_custody: receiving_custody_pda,
            collateral_custody: collateral_custody_pda,
            principal_custody: principal_custody_pda,
            oracle: oracle_pda,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::GetOpenPositionWithSwapAmountAndFees {
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
