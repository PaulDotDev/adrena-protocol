use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct RemovePool<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        mut,
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        close = transfer_authority,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    system_program: Program<'info, System>,
}

pub fn remove_pool<'info>(ctx: Context<'_, '_, '_, 'info, RemovePool<'info>>) -> Result<u8> {
    require!(
        ctx.accounts.pool.load()?.get_custodies().is_empty(),
        AdrenaError::InvalidPoolState
    );

    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.remove_pool(&ctx.accounts.pool.key())?;

    Ok(0)
}
