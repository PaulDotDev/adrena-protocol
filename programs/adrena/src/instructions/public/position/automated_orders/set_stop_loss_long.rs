use {
    crate::{
        error::AdrenaError,
        events::SetStopLossEvent,
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
pub struct SetStopLossLong<'info> {
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
        constraint = position.load()?.side == Side::Long as u8,
        has_one = owner,
        has_one = custody,
        has_one = pool,
    )]
    pub position: AccountLoader<'info, Position>,

    /// #5
    #[account(
        constraint = position.load()?.custody == custody.key(),
        constraint = position.load()?.collateral_custody == custody.key()
    )]
    pub custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct SetStopLossLongParams {
    // determine the trigger price
    pub stop_loss_limit_price: u64,
    // determine the actual close price (slippage)
    pub close_position_price: Option<u64>,
}

pub fn set_stop_loss_long<'info>(
    ctx: Context<'_, '_, '_, 'info, SetStopLossLong<'info>>,
    params: &SetStopLossLongParams,
) -> Result<()> {
    let mut position = ctx.accounts.position.load_mut()?;

    // Checks
    {
        if let Some(close_position_price) = params.close_position_price {
            if params.stop_loss_limit_price < close_position_price {
                return Err(ProgramError::InvalidArgument.into());
            }
        }
    }

    // Update state
    position.stop_loss_is_set = true as u8;
    position.stop_loss_limit_price = params.stop_loss_limit_price;
    position.stop_loss_close_position_price = params.close_position_price.unwrap_or(0);

    emit!(SetStopLossEvent {
        position_id: position.id,
        stop_loss_limit_price: params.stop_loss_limit_price,
        close_position_price: params.close_position_price,
        position_side: position.side,
    });

    Ok(())
}
