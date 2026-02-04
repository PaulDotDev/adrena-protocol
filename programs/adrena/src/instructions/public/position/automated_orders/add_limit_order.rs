use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            custody::Custody,
            limit_order_book::LimitOrderBook,
            pool::Pool,
            position::{Position, Side},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{Mint, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct AddLimitOrder<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = funding_account.mint == collateral_custody.load()?.mint,
        constraint = funding_account.owner == owner.key()
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

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
        mut,
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
        bump = limit_order_book.load()?.bump,
    )]
    pub limit_order_book: AccountLoader<'info, LimitOrderBook>,

    /// #7
    #[account(
        init_if_needed,
        payer = owner,
        token::mint = collateral_custody_mint,
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
    pub custody: AccountLoader<'info, Custody>,

    /// #10
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #11
    pub system_program: Program<'info, System>,

    /// #12
    pub token_program: Program<'info, Token>,

    /// #13
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct AddLimitOrderParams {
    pub trigger_price: u64,
    pub limit_price: Option<u64>,
    pub side: u8,
    pub amount: u64,
    pub leverage: u32,
}

pub fn add_limit_order<'info>(
    ctx: Context<'_, '_, '_, 'info, AddLimitOrder<'info>>,
    params: &AddLimitOrderParams,
) -> Result<u64> {
    // Preliminary checks
    {
        let side = Side::try_from(params.side)?;

        if side != Side::Long && side != Side::Short {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Check the custodies
        {
            if !ctx
                .accounts
                .pool
                .load()?
                .custodies
                .contains(&ctx.accounts.custody.key())
            {
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

            // When longing, should be the same custodies and not a stable custody
            if side == Side::Long
                && (ctx.accounts.collateral_custody.key() != ctx.accounts.custody.key()
                    || ctx.accounts.custody.load()?.is_stable())
            {
                return Err(ProgramError::InvalidArgument.into());
            }

            if side == Side::Short
                && ctx.accounts.collateral_custody.key() == ctx.accounts.custody.key()
            {
                return Err(ProgramError::InvalidArgument.into());
            }

            if side == Side::Short && !ctx.accounts.collateral_custody.load()?.is_stable() {
                return Err(ProgramError::InvalidArgument.into());
            }

            if side == Side::Short && ctx.accounts.custody.load()?.is_stable() {
                return Err(ProgramError::InvalidArgument.into());
            }
        }

        if params.trigger_price == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        if let Some(limit_price) = params.limit_price {
            if limit_price == 0 {
                return Err(ProgramError::InvalidArgument.into());
            }

            // We are long, need the limit_price to be bigger or equal to trigger_price
            if side == Side::Long && limit_price > params.trigger_price {
                return Err(ProgramError::InvalidArgument.into());
            }

            if side == Side::Short && limit_price < params.trigger_price {
                return Err(ProgramError::InvalidArgument.into());
            }
        }

        if params.amount == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        if params.leverage == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    let mut limit_order_book = ctx.accounts.limit_order_book.load_mut()?;

    let id = ctx.accounts.pool.load_mut()?.get_unique_limit_order_id();

    limit_order_book.add_limit_order(
        id,
        params.trigger_price,
        // 0 means no slippage
        params.limit_price.unwrap_or(0),
        ctx.accounts.custody.key(),
        ctx.accounts.collateral_custody.key(),
        params.side,
        params.amount,
        params.leverage,
    )?;

    ctx.accounts.cortex.load()?.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.collateral_escrow.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    // User pre-pay for the creation of the position + the execution fee
    // If less or none of the amount is used, will be refunded when the order is cancelled or executed
    {
        let escrowed_lamports =
            Rent::get()?.minimum_balance(Position::LEN) + Cortex::AUTOMATION_EXECUTION_FEE;

        limit_order_book.escrowed_lamports += escrowed_lamports;

        drop(limit_order_book);

        Cortex::transfer_sol(
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.limit_order_book.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            escrowed_lamports,
        )?;
    }

    Ok(id)
}
