use {
    crate::{
        error::AdrenaError,
        events::RemoveCollateralEvent,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{LeverageCheckType, Pool},
            position::{Position, Side},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct RemoveCollateralLong<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = receiving_account.mint == custody.load()?.mint,
        has_one = owner
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
        mut,
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
        seeds = [b"position",
                 owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        bump = position.load()?.bump,
        constraint = position.load()?.get_side() == Side::Long,
        has_one = owner,
        has_one = custody,
        has_one = pool,
    )]
    pub position: AccountLoader<'info, Position>,

    /// #7
    #[account(
        mut,
        constraint = position.load()?.custody == custody.key(),
        constraint = position.load()?.collateral_custody == custody.key()
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #8
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #9
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.token_account_bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #10
    pub adrena_program: Program<'info, Adrena>,

    /// #11
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RemoveCollateralLongParams {
    pub collateral_usd: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn remove_collateral_long(
    ctx: Context<RemoveCollateralLong>,
    params: &RemoveCollateralLongParams,
) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let mut position = ctx.accounts.position.load_mut()?;
    let pool = ctx.accounts.pool.load()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;

    let current_time = cortex.get_time()?;

    // Preliminary checks
    {
        require!(
            pool.is_trade_allowed() && custody.allow_trade(),
            AdrenaError::InstructionNotAllowed
        );

        if params.collateral_usd == 0 || params.collateral_usd >= position.collateral_usd {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let token_trade_price = oracle.get_oracle_price(custody.trade_oracle, current_time)?;
    let token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

    let max_collateral_price = token_price.high();

    let collateral =
        max_collateral_price.get_token_amount(params.collateral_usd, custody.decimals)?;

    if collateral > position.collateral_amount {
        return Err(ProgramError::InsufficientFunds.into());
    }

    msg!("Amount out: {}", collateral);

    // Update existing position
    {
        position.update_time = current_time;
        position.collateral_usd -= params.collateral_usd;
        position.collateral_amount -= collateral;
    }

    let leverage = pool.check_leverage(
        &position,
        &token_trade_price,
        &custody,
        &token_price,
        &custody,
        current_time,
        LeverageCheckType::RemoveCollateral,
    )?;

    cortex.transfer_tokens(
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        collateral,
    )?;

    // Update custody stats
    {
        custody.assets.collateral -= collateral;

        custody.long_positions.collateral_usd -= params.collateral_usd;
    }

    emit!(RemoveCollateralEvent {
        owner: ctx.accounts.owner.key(),
        position: ctx.accounts.position.key(),
        custody_mint: custody.mint.key(),
        side: position.side,
        remove_amount_usd: params.collateral_usd,
        new_collateral_amount_usd: position.collateral_usd,
        leverage: leverage as u32,
        position_id: position.id,
    });

    Ok(())
}
