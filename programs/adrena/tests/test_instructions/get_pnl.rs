use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::state::{
        chaos_labs_oracle::ChaosLabsBatchPrices, cortex::ProfitAndLoss, position::Position,
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

pub async fn get_pnl(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    position: &Pubkey,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<ProfitAndLoss, BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position).await;
    let custody_pda = position_account.custody;
    let collateral_custody_pda = position_account.collateral_custody;

    let result: ProfitAndLoss = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetPnl {
            cortex: cortex_pda,
            pool: *pool_pda,
            position: *position,
            custody: custody_pda,
            collateral_custody: collateral_custody_pda,
            oracle: oracle_pda,
        }
        .to_account_metas(Some(true)),
        adrena::instruction::GetPnl {
            params: adrena::instructions::GetPnlParams {
                oracle_prices: oracle_prices.clone(),
            },
        },
        payer,
        Some(get_update_pool_ix(program_test_ctx, payer, pool_pda, oracle_prices).await?),
        None,
    )
    .await?;

    // ==== THEN ==============================================================
    Ok(result)
}
