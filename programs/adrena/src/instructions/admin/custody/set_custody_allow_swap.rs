use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, custody::Custody, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetCustodyAllowSwap<'info> {
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
pub struct SetCustodyAllowSwapParams {
    pub allow_swap: bool,
}

pub fn set_custody_allow_swap<'info>(
    ctx: Context<'_, '_, '_, 'info, SetCustodyAllowSwap<'info>>,
    params: &SetCustodyAllowSwapParams,
) -> Result<()> {
    let mut custody = ctx.accounts.custody.load_mut()?;

    custody.allow_swap = params.allow_swap as u8;

    Ok(())
}
