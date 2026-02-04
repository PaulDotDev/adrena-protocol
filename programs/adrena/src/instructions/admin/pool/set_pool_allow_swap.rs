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
pub struct SetPoolAllowSwap<'info> {
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
pub struct SetPoolAllowSwapParams {
    pub allow_swap: bool,
}

pub fn set_pool_allow_swap<'info>(
    ctx: Context<'_, '_, '_, 'info, SetPoolAllowSwap<'info>>,
    params: &SetPoolAllowSwapParams,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Do not accept it when pool is still in Genesis
    require!(
        pool.get_liquidity_state()
            .ne(&PoolLiquidityState::GenesisLiquidity),
        AdrenaError::InvalidPoolLiquidityState
    );

    pool.allow_swap = if params.allow_swap { 1 } else { 0 };

    Ok(())
}
