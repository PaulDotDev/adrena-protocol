use {
    crate::{
        error::AdrenaError,
        events::CancelTakeProfitEvent,
        state::{cortex::Cortex, custody::Custody, pool::Pool, position::Position},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct CancelTakeProfit<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #3
    #[account(
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #4
    #[account(
        mut,
        seeds = [b"position",
                 owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        bump = position.load()?.bump,
        has_one = owner,
        has_one = custody,
        has_one = pool,
    )]
    pub position: AccountLoader<'info, Position>,

    /// #5
    #[account(
        constraint = position.load()?.custody == custody.key(),
    )]
    pub custody: AccountLoader<'info, Custody>,
}

pub fn cancel_take_profit<'info>(
    ctx: Context<'_, '_, '_, 'info, CancelTakeProfit<'info>>,
) -> Result<()> {
    let mut position = ctx.accounts.position.load_mut()?;

    if !position.take_profit_is_set() {
        return Ok(());
    }

    position.take_profit_is_set = false as u8;
    position.take_profit_limit_price = 0;

    emit!(CancelTakeProfitEvent {
        position_id: position.id,
        position_side: position.side,
    });

    Ok(())
}
