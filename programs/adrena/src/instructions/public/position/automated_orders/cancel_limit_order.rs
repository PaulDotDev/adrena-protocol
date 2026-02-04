use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, custody::Custody, limit_order_book::LimitOrderBook, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{Mint, Token, TokenAccount},
    },
    num::Zero,
};

#[derive(Accounts)]
pub struct CancelLimitOrder<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = receiving_account.mint == collateral_custody.load()?.mint,
        constraint = receiving_account.owner == owner.key()
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        mut,
        seeds = [b"limit_order_book",
                 owner.key().as_ref(),
                 pool.key().as_ref()],
        constraint = limit_order_book.load()?.is_initialized() @AdrenaError::InvalidLimitOrderState,
        bump = limit_order_book.load()?.bump,
    )]
    pub limit_order_book: AccountLoader<'info, LimitOrderBook>,

    /// #7
    #[account(
        mut,
        token::mint = collateral_custody.load()?.mint,
        token::authority = transfer_authority,
        seeds = [b"escrow_account",
        owner.key().as_ref(),
        pool.key().as_ref(),
        collateral_custody.load()?.mint.as_ref()],
        bump,
    )]
    pub collateral_escrow: Box<Account<'info, TokenAccount>>,

    /// #8
    #[account(
        constraint = collateral_custody_mint.key() == collateral_custody.load()?.mint,
    )]
    pub collateral_custody_mint: Account<'info, Mint>,

    /// #9
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #10
    pub system_program: Program<'info, System>,

    /// #11
    pub token_program: Program<'info, Token>,

    /// #12
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct CancelLimitOrderParams {
    pub id: u64,
}

pub fn cancel_limit_order(
    ctx: Context<CancelLimitOrder>,
    params: &CancelLimitOrderParams,
) -> Result<()> {
    let mut limit_order_book = ctx.accounts.limit_order_book.load_mut()?;

    // Preliminary checks
    {
        if !limit_order_book.is_initialized() {
            return Err(ProgramError::InvalidArgument.into());
        }

        if params.id == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        if !ctx
            .accounts
            .pool
            .load()?
            .custodies
            .contains(&ctx.accounts.collateral_custody.key())
        {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    let limit_order = limit_order_book.get_limit_order(params.id)?;

    if limit_order.collateral_custody != ctx.accounts.collateral_custody.key() {
        return Err(ProgramError::InvalidArgument.into());
    }

    // Transfer amount back to user from escrow account
    ctx.accounts.cortex.load()?.transfer_tokens(
        ctx.accounts.collateral_escrow.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        limit_order.amount,
    )?;

    limit_order_book.remove_limit_order(params.id)?;

    ctx.accounts.collateral_escrow.reload()?;

    // Close the escrow account if there are no more need for it
    if !limit_order_book.is_collateral_escrowed(&ctx.accounts.collateral_custody.key()) {
        // Account still have some dust, transfer it to owner
        if !ctx.accounts.collateral_escrow.amount.is_zero() {
            ctx.accounts.cortex.load()?.transfer_tokens(
                ctx.accounts.collateral_escrow.to_account_info(),
                ctx.accounts.receiving_account.to_account_info(),
                ctx.accounts.transfer_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.collateral_escrow.amount,
            )?;
        }

        // Then close the account and return rent to owner
        Cortex::close_token_account(
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.collateral_escrow.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            &[&[
                b"transfer_authority",
                &[ctx.accounts.cortex.load()?.transfer_authority_bump],
            ]],
        )?;
    }

    // Delete the limit order book if there are no more limit orders
    if limit_order_book.registered_limit_order_count == 0 {
        // Reset in case removing SOL doesn't impact the account right away
        *limit_order_book = LimitOrderBook::default();

        Cortex::transfer_sol_from_owned(
            ctx.accounts.limit_order_book.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.limit_order_book.get_lamports(),
        )?;
    }

    Ok(())
}
