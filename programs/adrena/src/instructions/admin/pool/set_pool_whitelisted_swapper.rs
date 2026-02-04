use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetPoolWhitelistedSwapper<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(
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

    /// #4
    /// CHECK: Any account to be registered as a whitelisted swapper
    pub whitelisted_swapper: AccountInfo<'info>,
}

pub fn set_pool_whitelisted_swapper(ctx: Context<SetPoolWhitelistedSwapper>) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Only accepts Active to Idle and vice versa

    msg!(
        "Previous pool whitelist swapper: {:?}",
        pool.whitelisted_swapper
    );

    pool.whitelisted_swapper = ctx.accounts.whitelisted_swapper.key();

    msg!("New pool whitelist swapper: {:?}", pool.whitelisted_swapper);

    Ok(())
}
