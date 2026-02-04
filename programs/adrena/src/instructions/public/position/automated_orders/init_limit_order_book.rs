use {
    crate::{
        error::AdrenaError,
        state::{limit_order_book::LimitOrderBook, pool::Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct InitLimitOrderBook<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #3
    #[account(
        init,
        space = LimitOrderBook::LEN,
        payer = owner,
        seeds = [b"limit_order_book",
                 owner.key().as_ref(),
                 pool.key().as_ref()],
        bump,
    )]
    pub limit_order_book: AccountLoader<'info, LimitOrderBook>,

    /// #4
    pub system_program: Program<'info, System>,
}

pub fn init_limit_order_book(ctx: Context<InitLimitOrderBook>) -> Result<()> {
    let mut limit_order_book = ctx.accounts.limit_order_book.load_init()?;

    limit_order_book.owner = *ctx.accounts.owner.key;
    limit_order_book.initialized = true as u8;
    limit_order_book.bump = ctx.bumps.limit_order_book;

    Ok(())
}
