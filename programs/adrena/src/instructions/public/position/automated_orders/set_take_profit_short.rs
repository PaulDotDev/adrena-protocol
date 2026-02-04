use {
    crate::{
        error::AdrenaError,
        events::SetTakeProfitEvent,
        state::{
            cortex::Cortex,
            custody::Custody,
            pool::Pool,
            position::{Position, Side},
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetTakeProfitShort<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
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
        seeds = [b"position",
                 owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        bump = position.load()?.bump,
        constraint = position.load()?.get_side() == Side::Short,
        has_one = owner,
        has_one = custody,
        has_one = pool,
    )]
    pub position: AccountLoader<'info, Position>,

    /// #5
    #[account(
        mut,
        constraint = position.load()?.custody == custody.key()
    )]
    pub custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct SetTakeProfitShortParams {
    pub take_profit_limit_price: u64,
}

pub fn set_take_profit_short<'info>(
    ctx: Context<'_, '_, '_, 'info, SetTakeProfitShort<'info>>,
    params: &SetTakeProfitShortParams,
) -> Result<()> {
    let mut position = ctx.accounts.position.load_mut()?;

    // Update state
    position.take_profit_is_set = true as u8;
    position.take_profit_limit_price = params.take_profit_limit_price;

    emit!(SetTakeProfitEvent {
        position_id: position.id,
        take_profit_limit_price: params.take_profit_limit_price,
        position_side: position.side,
    });

    Ok(())
}
