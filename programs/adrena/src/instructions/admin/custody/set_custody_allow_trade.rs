use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, custody::Custody, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetCustodyAllowTrade<'info> {
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

    /// #4
    #[account(
        mut,
        seeds = [b"custody",
                pool.key().as_ref(),
                custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump,
    )]
    pub custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetCustodyAllowTradeParams {
    pub allow_trade: bool,
}

pub fn set_custody_allow_trade<'info>(
    ctx: Context<'_, '_, '_, 'info, SetCustodyAllowTrade<'info>>,
    params: &SetCustodyAllowTradeParams,
) -> Result<()> {
    let mut custody = ctx.accounts.custody.load_mut()?;

    custody.allow_trade = params.allow_trade as u8;

    Ok(())
}
