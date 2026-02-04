use {
    super::{open_position_long::OpenPositionLongParams, IncreasePositionLongParams},
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool, position::Position,
        },
        SwapParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct OpenOrIncreasePositionWithSwapLong<'info> {
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
        constraint = collateral_account.mint == principal_custody.load()?.mint,
        has_one = owner
    )]
    pub collateral_account: Box<Account<'info, TokenAccount>>,

    /// #5
    // i.e in case of open short position on ETH with BTC, receiving custody is BTC
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
    // i.e in case of open short position on ETH with BTC, principal custody is ETH
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 principal_custody.load()?.mint.as_ref()],
        bump = principal_custody.load()?.bump
    )]
    pub principal_custody: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 principal_custody.load()?.mint.as_ref()],
        bump = principal_custody.load()?.token_account_bump
    )]
    pub principal_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #10
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #11
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #12
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #13
    /// CHECK: initialized by CPI open_position
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// #14
    pub system_program: Program<'info, System>,

    /// #15
    pub token_program: Program<'info, Token>,

    /// #16
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenPositionWithSwapParams {
    pub price: u64,
    pub collateral: u64,
    pub leverage: u32, // in BPS
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn open_or_increase_position_with_swap_long(
    ctx: Context<OpenOrIncreasePositionWithSwapLong>,
    params: &OpenPositionWithSwapParams,
) -> Result<()> {
    let cortex: std::cell::Ref<'_, Cortex> = ctx.accounts.cortex.load()?;
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
        .ne(&ctx.accounts.principal_custody.key());

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
            ctx.accounts.principal_custody.to_account_info(),
            ctx.accounts
                .principal_custody_token_account
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

    if existing_position {
        cortex_acc.internal_increase_position_long(
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
            ctx.accounts
                .principal_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            IncreasePositionLongParams {
                price: params.price,
                collateral: collateral_amount_post_swap,
                leverage: params.leverage,
                oracle_prices: None,
            },
        )?;
    } else {
        cortex_acc.internal_open_position_long(
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
            ctx.accounts
                .principal_custody_token_account
                .to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            OpenPositionLongParams {
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
