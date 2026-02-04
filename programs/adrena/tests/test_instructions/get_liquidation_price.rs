use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::GetLiquidationPriceParams,
        state::{
            custody::Custody,
            position::{Position, Side},
        },
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_liquidation_price(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    position_owner: &Pubkey,
    custody_token_mint: &Pubkey,
    side: Side,
    params: GetLiquidationPriceParams,
) -> std::result::Result<u64, BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let (position_pda, _) = pda::get_position_pda(position_owner, pool_pda, &custody_pda, side);

    let position_account = utils::get_account::<Position>(program_test_ctx, position_pda).await;
    let collateral_custody_pda = position_account.collateral_custody;

    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, custody_pda).await;
    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;

    let custody_pda = pda::get_custody_pda(pool_pda, &custody_account.mint).0;
    let collateral_custody_pda = pda::get_custody_pda(pool_pda, &collateral_custody_account.mint).0;

    let result: u64 = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        adrena::accounts::GetLiquidationPrice {
            cortex: cortex_pda,
            pool: *pool_pda,
            position: position_pda,
            custody: custody_pda,
            collateral_custody: collateral_custody_pda,
            oracle: oracle_pda,
        }
        .to_account_metas(None),
        adrena::instruction::GetLiquidationPrice {
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
