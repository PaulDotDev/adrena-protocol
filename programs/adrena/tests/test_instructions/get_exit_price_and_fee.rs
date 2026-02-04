use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::GetExitPriceAndFeeParams,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::ExitPriceAndFee, custody::Custody,
            position::Position,
        },
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_exit_price_and_fee(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    position_pda: &Pubkey,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<ExitPriceAndFee, BanksClientError> {
    // ==== WHEN ==============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let oracle_pda = pda::get_oracle_pda().0;

    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;
    let custody_pda = position_account.custody;
    let collateral_custody_pda = position_account.collateral_custody;

    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, custody_pda).await;
    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;

    let custody_pda = pda::get_custody_pda(pool_pda, &custody_account.mint).0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, &collateral_custody_account.mint).0;

    let result: ExitPriceAndFee = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetExitPriceAndFee {
            cortex: cortex_pda,
            pool: *pool_pda,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_custody: collateral_custody_pda,
        }
        .to_account_metas(None),
        adrena::instruction::GetExitPriceAndFee {
            params: GetExitPriceAndFeeParams {
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
