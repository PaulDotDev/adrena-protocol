use {
    super::{open_position_short::OpenPositionShortParams, IncreasePositionShortParams},
    crate::{
        error::AdrenaError,
        open_or_increase_position_with_swap_long::OpenPositionWithSwapParams,
        program::Adrena,
        state::{cortex::Cortex, custody::Custody, oracle::Oracle, pool::Pool, position::Position},
        SwapParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct OpenOrIncreasePositionWithSwapShort<'info> {
    /// #1
    pub owner: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        constraint = funding_account.mint == receiving_custody.load()?.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #4
    // used as temporary location to store collateral between the swap and the open position
    // i.e in case of open short position on ETH with BTC, collateral account is storing stable (USDC, USDT etc.)
    #[account(
        mut,
        constraint = collateral_account.mint == collateral_custody.load()?.mint,
        has_one = owner
    )]
    pub collateral_account: Box<Account<'info, TokenAccount>>,

    /// #5
    // i.e in case of open short position on ETH with BTC, receiving custody is ETH
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 receiving_custody.load()?.mint.as_ref()],
        bump = receiving_custody.load()?.bump
    )]
    pub receiving_custody: AccountLoader<'info, Custody>,

    /// #6
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #7
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 receiving_custody.load()?.mint.as_ref()],
        bump = receiving_custody.load()?.token_account_bump
    )]
    pub receiving_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #8
    // i.e in case of open short position on ETH with BTC, collateral custody is a stable (USDC, USDT etc.)
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.bump
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.token_account_bump
    )]
    pub collateral_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #10
    // i.e in case of open short position on ETH with BTC, principal custody is BTC
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 principal_custody.load()?.mint.as_ref()],
        bump = principal_custody.load()?.bump
    )]
    pub principal_custody: AccountLoader<'info, Custody>,

    /// #11
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 principal_custody.load()?.mint.as_ref()],
        bump = principal_custody.load()?.token_account_bump
    )]
    pub principal_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #12
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #13
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #14
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #15
    /// CHECK: initialized by CPI open_position
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// #16
    pub system_program: Program<'info, System>,

    /// #17
    pub token_program: Program<'info, Token>,

    /// #18
    pub adrena_program: Program<'info, Adrena>,
}

pub fn open_or_increase_position_with_swap_short(
    ctx: Context<OpenOrIncreasePositionWithSwapShort>,
    params: &OpenPositionWithSwapParams,
) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let current_time = cortex.get_time()?;

    // Preliminary checks
    {
        let pool = ctx.accounts.pool.load()?;
        let custody = ctx.accounts.receiving_custody.load()?;

        require!(
            pool.is_trade_allowed() && pool.is_swap_allowed() && custody.allow_trade(),
            AdrenaError::InstructionNotAllowed
        );

        let mut oracle = ctx.accounts.oracle.load_mut()?;

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let swap_required = ctx
        .accounts
        .receiving_custody
        .key()
        .ne(&ctx.accounts.collateral_custody.key());

    let cortex_acc = *cortex;

    drop(cortex);

    let collateral_amount_post_swap = if swap_required {
        let collateral_amount_before = ctx.accounts.collateral_account.amount;

        cortex_acc.internal_swap(
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts.collateral_account.to_account_info(),
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.receiving_custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts
                .receiving_custody_token_account
                .to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            SwapParams {
                amount_in: params.collateral,
                min_amount_out: 0,
                oracle_prices: None,
            },
        )?;

        // Reload accounts so they are up to date after what happened in the cpi
        {
            ctx.accounts.receiving_custody_token_account.reload()?;
            ctx.accounts.collateral_account.reload()?;
        }

        let collateral_amount_after = ctx.accounts.collateral_account.amount;
        let collateral_amount = collateral_amount_after - collateral_amount_before;

        msg!("Swapped for {} tokens", collateral_amount);

        collateral_amount
    } else {
        msg!("No swap required");

        params.collateral
    };

    // Load position if it exists
    let position: Option<Position> = {
        let position_data = &ctx.accounts.position.try_borrow_mut_data()?;

        let position: Option<Position> = if position_data.len() == 0 {
            None
        } else {
            let mut position_data = &position_data[..];

            match Position::try_deserialize(&mut position_data) {
                Ok(position) => Some(position),
                Err(_) => None,
            }
        };

        position
    };

    if position.is_some() && position.unwrap().size_usd > 0 {
        cortex_acc.internal_increase_position_short(
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.collateral_account.to_account_info(),
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.principal_custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            IncreasePositionShortParams {
                price: params.price,
                collateral: collateral_amount_post_swap,
                leverage: params.leverage,
                oracle_prices: None,
            },
        )?;
    } else {
        cortex_acc.internal_open_position_short(
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.collateral_account.to_account_info(),
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.principal_custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            OpenPositionShortParams {
                price: params.price,
                collateral: collateral_amount_post_swap,
                leverage: params.leverage,
                oracle_prices: None,
            },
        )?;
    }

    // Reload accounts so they are up to date after what happened in the cpi
    {
        ctx.accounts.collateral_account.reload()?;
    }

    Ok(())
}
