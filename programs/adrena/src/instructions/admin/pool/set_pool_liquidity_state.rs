use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            pool::{Pool, PoolLiquidityState},
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetPoolLiquidityState<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #3
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetPoolLiquidityStateParams {
    pub liquidity_state: u8,
}

pub fn set_pool_liquidity_state(
    ctx: Context<SetPoolLiquidityState>,
    params: &SetPoolLiquidityStateParams,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Only accepts Active to Idle and vice versa

    let old_liquidity_state = pool.get_liquidity_state();
    let new_liquidity_state = PoolLiquidityState::try_from(params.liquidity_state)?;

    require!(
        (old_liquidity_state.eq(&PoolLiquidityState::Active)
            && new_liquidity_state.eq(&PoolLiquidityState::Idle))
            || (old_liquidity_state.eq(&PoolLiquidityState::Idle)
                && new_liquidity_state.eq(&PoolLiquidityState::Active)),
        AdrenaError::InvalidPoolLiquidityState
    );

    pool.liquidity_state = params.liquidity_state;

    Ok(())
}
