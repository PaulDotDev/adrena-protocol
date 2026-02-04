use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            limit_order_book::LimitOrderBook,
            oracle::Oracle,
            pool::Pool,
            position::{Position, Side},
        },
        IncreasePositionShortParams, OpenPositionShortParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
    num::Zero,
};

#[derive(Accounts)]
pub struct ExecuteLimitOrderShort<'info> {
    /// #1
    /// CHECK: Checked later
    #[account(mut)]
    pub owner: AccountInfo<'info>,

    /// #2
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #3
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

    /// #4
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #5
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #6
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.bump
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #7
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.token_account_bump
    )]
    pub collateral_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #8
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #9
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #10
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #11
    /// CHECK: initialized by CPI open_position, deleted by CPI close_position
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// #12
    #[account(
        mut,
        seeds = [b"limit_order_book",
            owner.key().as_ref(),
            pool.key().as_ref()],
        constraint = limit_order_book.load()?.is_initialized() @AdrenaError::InvalidLimitOrderState,
        bump = limit_order_book.load()?.bump,
    )]
    pub limit_order_book: AccountLoader<'info, LimitOrderBook>,

    /// #13
    pub system_program: Program<'info, System>,

    /// #14
    pub token_program: Program<'info, Token>,

    /// #15
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ExecuteLimitOrderShortParams {
    pub id: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn execute_limit_order_short(
    ctx: Context<ExecuteLimitOrderShort>,
    params: &ExecuteLimitOrderShortParams,
) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Preliminary checks
    {
        let pool = ctx.accounts.pool.load()?;
        let custody = ctx.accounts.custody.load()?;

        require!(
            pool.is_trade_allowed() && custody.allow_trade(),
            AdrenaError::InstructionNotAllowed
        );

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    // Look which limit order is executable for the custody and current price
    let token_trade_price =
        oracle.get_oracle_price(ctx.accounts.custody.load()?.trade_oracle, current_time)?;

    drop(oracle);

    let limit_order_book = ctx.accounts.limit_order_book.load()?;

    let limit_order = *limit_order_book.get_limit_order(params.id)?;

    drop(limit_order_book);

    require!(
        limit_order.is_executable(&token_trade_price, &ctx.accounts.custody.key())
            && limit_order.get_side() == Side::Short
            && limit_order.collateral_custody == ctx.accounts.collateral_custody.key(),
        AdrenaError::InstructionNotAllowed
    );

    let caller_lamports_before = ctx.accounts.caller.lamports();

    // Load position if it exists
    let existing_position: bool = {
        let position_data = &ctx.accounts.position.try_borrow_mut_data()?;

        if position_data.len() == 0 {
            false
        } else {
            let mut position_data = &position_data[..];

            match Position::try_deserialize(&mut position_data) {
                Ok(position) => position.size_usd > 0,
                Err(_) => false,
            }
        }
    };

    let cortex_acc = *ctx.accounts.cortex.load()?;

    if existing_position {
        cortex_acc.internal_increase_position_short(
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.caller.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.collateral_escrow.to_account_info(),
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            IncreasePositionShortParams {
                price: token_trade_price.price,
                collateral: limit_order.amount,
                leverage: limit_order.leverage,
                oracle_prices: None,
            },
        )?;
    } else {
        cortex_acc.internal_open_position_short(
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.caller.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.collateral_escrow.to_account_info(),
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            OpenPositionShortParams {
                price: token_trade_price.price,
                collateral: limit_order.amount,
                leverage: limit_order.leverage,
                oracle_prices: None,
            },
        )?;
    }

    // Reload accounts so they are up to date after what happened in the cpi
    ctx.accounts.collateral_escrow.reload()?;

    let caller_lamports_after = ctx.accounts.caller.lamports();

    let mut limit_order_book = ctx.accounts.limit_order_book.load_mut()?;

    limit_order_book.remove_limit_order(params.id)?;

    // Close the escrow account if there are no more need for it
    if !limit_order_book.is_collateral_escrowed(&ctx.accounts.collateral_custody.key()) {
        // Account still have some dust, transfer it to the pool
        if !ctx.accounts.collateral_escrow.amount.is_zero() {
            {
                let mut collateral_custody = ctx.accounts.collateral_custody.load_mut()?;
                collateral_custody.assets.owned += ctx.accounts.collateral_escrow.amount;

                collateral_custody.update_borrow_rate(ctx.accounts.cortex.load()?.get_time()?)?;
            }

            ctx.accounts.cortex.load()?.transfer_tokens(
                ctx.accounts.collateral_escrow.to_account_info(),
                ctx.accounts
                    .collateral_custody_token_account
                    .to_account_info(),
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

    // Repay the caller for the lamports spent during increase/open + reward an execution fee
    {
        let mut lamports_to_pay_caller =
            (caller_lamports_before - caller_lamports_after) + Cortex::AUTOMATION_EXECUTION_FEE;

        // If there is no money, don't pay full amount - we still want the order to be executed
        lamports_to_pay_caller = lamports_to_pay_caller.min(limit_order_book.escrowed_lamports);

        Cortex::transfer_sol_from_owned(
            ctx.accounts.limit_order_book.to_account_info(),
            ctx.accounts.caller.to_account_info(),
            lamports_to_pay_caller,
        )?;

        limit_order_book.escrowed_lamports -= lamports_to_pay_caller;
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
