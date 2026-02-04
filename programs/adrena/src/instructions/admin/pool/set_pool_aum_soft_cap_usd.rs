use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetPoolAumSoftCapUsd<'info> {
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
pub struct SetPoolAumSoftCapUsdParams {
    pub aum_soft_cap_usd: u64,
}

pub fn set_pool_aum_soft_cap_usd<'info>(
    ctx: Context<'_, '_, '_, 'info, SetPoolAumSoftCapUsd<'info>>,
    params: &SetPoolAumSoftCapUsdParams,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    pool.aum_soft_cap_usd = params.aum_soft_cap_usd;

    msg!("Apply new aum_soft_cap_usd: {}", pool.aum_soft_cap_usd);

    Ok(())
}
