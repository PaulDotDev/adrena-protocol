use {
    crate::{
        error::AdrenaError,
        events::OpenPositionEvent,
        math,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{LeverageCheckType, Pool},
            position::{Position, Side},
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct OpenPositionLong<'info> {
    /// #1 Must be signer or not depending
    /// if the caller is the transfer_authority (internal call for limit order) or the owner
    ///
    /// CHECK: This is validated dynamically in the handler
    pub owner: AccountInfo<'info>,

    /// #2
    pub caller: Signer<'info>,

    /// #3
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #4
    #[account(
        mut,
        constraint = funding_account.mint == custody.load()?.mint,
        // Ownership depends on the caller, if the caller is transfer_authority,
        // then the owner is the program (limit order -> escrowed collateral account)
        // otherwise the owner is the user
        //
        // This is validated dynamically in the handler
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #5
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #6
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #8
    #[account(
        init,
        payer = payer,
        space = Position::LEN,
        seeds = [b"position",
                 owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[Side::Long as u8]],
        bump
    )]
    pub position: AccountLoader<'info, Position>,

    /// #9
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #10
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #11
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.token_account_bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #12
    pub system_program: Program<'info, System>,

    /// #13
    pub token_program: Program<'info, Token>,

    /// #14
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenPositionLongParams {
    pub price: u64, // requested. used for slippage protection
    pub collateral: u64,
    pub leverage: u32, // in BPS
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

// Note: On long positions, the custody and "collateral custody" are the same
pub fn open_position_long(
    ctx: Context<OpenPositionLong>,
    params: &OpenPositionLongParams,
) -> Result<()> {
    msg!(
        "price: {}, collateral: {}, leverage: {}",
        params.price,
        params.collateral,
        params.leverage
    );
    let mut cortex = ctx.accounts.cortex.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let mut position = ctx.accounts.position.load_init()?;
    let pool = ctx.accounts.pool.load()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = cortex.get_time()?;

    let custody_key = ctx.accounts.custody.key();
    let custody_mint = custody.mint;

    // Preliminary checks
    {
        require!(
            pool.is_trade_allowed() && custody.allow_trade(),
            AdrenaError::InstructionNotAllowed
        );

        if params.price == 0 || params.collateral == 0 || params.leverage == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Check that custody is not a stable coin
        require!(!custody.is_stable(), AdrenaError::InvalidCustody);

        if ctx.accounts.caller.key() == ctx.accounts.transfer_authority.key() {
            // This is an internal call (limit order)

            // Owner of the token account must be the program (escrowed collateral account for limit order)
            if !ctx
                .accounts
                .funding_account
                .owner
                .eq(&ctx.accounts.transfer_authority.key())
            {
                return Err(ProgramError::InvalidArgument.into());
            }
        } else {
            // This is a user call
            // Owner must sign and own the funding token account

            if !ctx.accounts.owner.is_signer {
                return Err(ProgramError::InvalidArgument.into());
            }

            if !ctx
                .accounts
                .funding_account
                .owner
                .eq(&ctx.accounts.owner.key())
            {
                return Err(ProgramError::InvalidArgument.into());
            }
        }

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let token_trade_price = oracle.get_oracle_price(custody.trade_oracle, current_time)?;
    let collateral_token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

    let collateral_token_price_high = collateral_token_price.high();
    let collateral_token_price_low = collateral_token_price.low();
    msg!(
        "Collateral price: {} (high {}, low {})",
        collateral_token_price.price,
        collateral_token_price_high.price,
        collateral_token_price_low.price
    );
    msg!("Trade price: {}", token_trade_price.price);

    // Check slippage - Here we want to protect the user opening at a disadvantageous price, based on his chosen direction
    // but we want to still open the position if the price is in his favor (which is opinionated, but as users ourselves, we prefer this)
    require_gte!(
        params.price,
        token_trade_price.price,
        AdrenaError::MaxPriceSlippage
    );

    // Calculate amounts
    #[allow(clippy::type_complexity)]
    let (
        size_usd,
        collateral,
        collateral_usd,
        locked_amount,
        borrow_size_usd,
        exit_fee_usd,
        liquidation_fee_usd,
    ) = (|| -> Result<(u64, u64, u64, u64, u64, u64, u64)> {
        let collateral = params.collateral;
        let collateral_usd =
            collateral_token_price_low.get_asset_amount_usd(collateral, custody.decimals)?;

        require!(
            collateral_usd >= Cortex::POSITION_MIN_COLLATERAL_VALUE,
            AdrenaError::InsufficientCollateral
        );

        // In collateral
        let size =
            math::checked_as_u64(collateral as u128 * params.leverage as u128 / Cortex::BPS_POWER)?;

        let size_usd = collateral_token_price_low.get_asset_amount_usd(size, custody.decimals)?;

        // Calculate OUT fees (estimated, unrealized). Because we cannot know in advance the pyth confidence (token_high_price) at exit time yet
        let exit_fee_usd = {
            let exit_fee = pool.get_exit_fee(size, &custody)?;

            collateral_token_price.get_asset_amount_usd(exit_fee, custody.decimals)?
        };
        // (estimated, unrealized). Because we cannot know in advance the pyth confidence (token_high_price) at exit time yet
        let liquidation_fee_usd = {
            let liquidation_fee: u64 = pool.get_liquidation_fee(size, &custody)?;

            collateral_token_price.get_asset_amount_usd(liquidation_fee, custody.decimals)?
        };

        let locked_amount = size;

        let borrow_size_usd = size_usd;

        Ok((
            size_usd,
            collateral,
            collateral_usd,
            locked_amount,
            borrow_size_usd,
            exit_fee_usd,
            liquidation_fee_usd,
        ))
    })()?;

    {
        msg!("Initialize new position");

        position.bump = ctx.bumps.position;
        position.owner = ctx.accounts.owner.key();
        position.pool = ctx.accounts.pool.key();
        position.custody = custody_key;
        position.collateral_custody = custody_key;
        position.open_time = current_time;
        position.update_time = 0;
        position.side = Side::Long.into();

        // Position price used to calculate PnL later on
        // Based on custody_trade_price
        position.price = token_trade_price.price;

        position.size_usd = size_usd;
        position.borrow_size_usd = borrow_size_usd;
        position.collateral_usd = collateral_usd;
        position.unrealized_interest_usd = 0;
        position.cumulative_interest_snapshot =
            U128Split::from(custody.get_cumulative_interest(current_time)?);
        // In collateral amount
        position.locked_amount = locked_amount;
        position.collateral_amount = collateral;
        position.exit_fee_usd = exit_fee_usd;
        position.liquidation_fee_usd = liquidation_fee_usd;
        // assign unique ID
        position.id = cortex.get_unique_position_id();
        position.stop_loss_limit_price = 0;
        position.stop_loss_close_position_price = 0;
        position.take_profit_limit_price = 0;
        position.take_profit_is_set = 0;
        position.stop_loss_is_set = 0;
        position.paid_interest_usd = 0;
    }

    let leverage = pool.check_leverage(
        &position,
        &token_trade_price,
        &custody,
        &collateral_token_price,
        &custody,
        current_time,
        LeverageCheckType::Initial,
    )?;

    // Lock funds for potential profit payoff according to calculated locked_amount
    custody.lock_funds(position.locked_amount)?;

    // Take user collateral
    if ctx.accounts.caller.key() == ctx.accounts.transfer_authority.key() {
        cortex.transfer_tokens(
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts.custody_token_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.collateral,
        )?;
    } else {
        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts.custody_token_account.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.collateral,
        )?;
    }

    // Update custody stats
    {
        // Here we account for the whole amount brought by the user, without fee. This becomes available to others.
        // Because this collateral has been used to open the borrowed position, and any fees/borrow/loss/profits will be resolved against this "IoU".
        custody.assets.collateral += collateral;

        custody.volume_stats.open_position_usd = custody
            .volume_stats
            .open_position_usd
            .wrapping_add(size_usd);

        custody.trade_stats.oi_long_usd += size_usd;

        custody.update_accounting_after_open_position_long(
            &position,
            &collateral_token_price_high,
            current_time,
        )?;

        custody.update_borrow_rate(current_time)?;
    }

    emit!(OpenPositionEvent {
        owner: ctx.accounts.owner.key(),
        position: ctx.accounts.position.key(),
        custody_mint,
        side: position.side,
        size_usd,
        price: token_trade_price.price,
        collateral_amount_usd: position.collateral_usd,
        leverage: leverage as u32,
        position_id: position.id,
    });

    Ok(())
}
