use {
    crate::utils::{self, pda},
    adrena::{
        instructions::SetPoolLiquidityStateParams,
        state::pool::{Pool, PoolLiquidityState},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_pool_liquidity_state(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    liquidity_state: PoolLiquidityState,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetPoolLiquidityState {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
        }
        .to_account_metas(None),
        adrena::instruction::SetPoolLiquidityState {
            params: SetPoolLiquidityStateParams {
                liquidity_state: liquidity_state.into(),
            },
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("set_pool_liquidity_state", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, *pool_pda).await;

    assert_eq!(pool_account.get_liquidity_state(), liquidity_state);

    Ok(())
}
