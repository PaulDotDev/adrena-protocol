use {
    crate::{
        error::AdrenaError,
        events::ClosePositionEvent,
        math,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
            position::{Position, Side, MIN_POSITION_OPEN_TIME_SECONDS},
            user_profile::UserProfile,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct ClosePositionShort<'info> {
    /// #1
    /// CHECK: Checked within the instruction
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #2
    /// CHECK: verified through equality with caller (if permissioned)
    #[account(mut)]
    pub owner: AccountInfo<'info>,

    /// #3
    #[account(
        mut,
        constraint = receiving_account.mint == collateral_custody.load()?.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// #4
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #5
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #6
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #7
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
        has_one = collateral_custody,
        has_one = pool,
        // Close the PDA only if closing 100% of the position
    )]
    pub position: AccountLoader<'info, Position>,

    /// #8
    #[account(
        mut,
        constraint = position.load()?.custody == custody.key()
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #10
    #[account(
        mut,
        constraint = position.load()?.collateral_custody == collateral_custody.key()
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #11
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.token_account_bump
    )]
    pub collateral_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #12
    #[account(
        mut,
        seeds = [b"user_profile",
                 owner.key().as_ref()],
        bump = user_profile.load()?.bump
    )]
    pub user_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #13
    #[account(mut)]
    pub referrer_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #14
    pub token_program: Program<'info, Token>,

    /// #15
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ClosePositionShortParams {
    pub price: Option<u64>,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
    // Amount of size to close in bps, 10000 = 1%, 1000000 = 100%
    pub percentage: u64,
}

pub fn close_position_short(
    ctx: Context<ClosePositionShort>,
    params: &ClosePositionShortParams,
) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let mut position = ctx.accounts.position.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let mut collateral_custody = ctx.accounts.collateral_custody.load_mut()?;
    let custody_mint = custody.mint;
    let is_partial_close = params.percentage < (Cortex::BPS_POWER * 100) as u64;

    let current_time = cortex.get_time()?;

    // Preliminary checks
    let has_referrer = {
        // Check that the position has been open for at least MIN_POSITION_OPEN_TIME_SECONDS
        require!(
            current_time >= position.open_time + MIN_POSITION_OPEN_TIME_SECONDS as i64,
            AdrenaError::PositionTooYoung
        );

        // Price cannot be 0, as 0 is used to indicate no price limit internally
        if params.price.is_some() && params.price.unwrap() == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Accept percentages between 1% and 100%
        let valid_range = (Cortex::BPS_POWER as u64)..=(Cortex::BPS_POWER as u64 * 100);

        if !valid_range.contains(&params.percentage) {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Check the referrer
        if let Some(user_profile) = ctx.accounts.user_profile.as_ref() {
            if let Some(referrer_profile) = ctx.accounts.referrer_profile.as_ref() {
                // the referrer should be the same as in the profile
                require!(
                    user_profile.load()?.referrer_profile == referrer_profile.key(),
                    AdrenaError::MissingOrInvalidReferrerAccount
                );

                true
            } else {
                // If there is a referrer in the profile, there should be a referrer account
                require!(
                    user_profile.load()?.referrer_profile == Pubkey::default(),
                    AdrenaError::MissingOrInvalidReferrerAccount
                );

                false
            }
        } else {
            // No referrer if no user profile
            false
        }
    };

    // TODO
    // Load the oracle prices, while enforcing pixel perfect mechanism for SL/TP:
    //
    // The instruction can be called with an old price, where the position was eligible for SL/TP.
    // This mechanism allows to alleviate the pain of congestion and transaction landing.
    // If the position wasn't SL/TPed timely, we will still SL/TP it at the right price even minutes later.
    // Benefitting the trader, while not hurting the protocol.
    let (token_trade_price, collateral_token_price) = {
        let mut oracle = ctx.accounts.oracle.load_mut()?;

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }

        let token_trade_price = oracle.get_oracle_price(custody.trade_oracle, current_time)?;
        let collateral_token_price =
            oracle.get_oracle_price(collateral_custody.oracle, current_time)?;

        (token_trade_price, collateral_token_price)
    };

    let mut pool = ctx.accounts.pool.load_mut()?;

    let collateral_token_price_high = collateral_token_price.high();
    let collateral_token_price_low = collateral_token_price.low();
    msg!(
        "Collateral price: {} (high {}, low {})",
        collateral_token_price.price,
        collateral_token_price_high.price,
        collateral_token_price_low.price
    );
    msg!("Trade price: {}", token_trade_price.price);

    // The instruction can be called permissionlessly IF the SL or TP are set and their limit price is reached
    if !ctx.accounts.caller.key.eq(ctx.accounts.owner.key) {
        require!(
            (position.take_profit_is_set()
                && position.take_profit_reached(token_trade_price.price))
                || (position.stop_loss_is_set()
                    && position.stop_loss_reached(token_trade_price.price)),
            AdrenaError::InvalidPositionState
        );

        require!(
            position.stop_loss_slippage_ok(token_trade_price.price),
            AdrenaError::MaxPriceSlippage
        );

        // Forbid partial SL/TP for now
        require!(!is_partial_close, AdrenaError::InvalidPositionState);
    } else {
        // If the caller is the owner, the slippage mandatory
        require!(
            params.price.is_some(),
            AdrenaError::MissingClosePositionPrice
        );
    }

    // Check exit price slippage
    {
        let exit_price = token_trade_price.price;

        msg!("Exit price: {}", exit_price);

        if let Some(price) = params.price {
            msg!("Params.price: {}", price);

            require_gte!(price, exit_price, AdrenaError::MaxPriceSlippage);
        }
    }

    // Clone the position and downscale it to calculate exit numbers for partial close
    let mut position_to_close = *position;

    if is_partial_close {
        position_to_close.downscale(math::checked_as_u64(
            100_u128 * Cortex::BPS_POWER - params.percentage as u128,
        )?)?;
    }

    let exit_numbers = pool.get_exit_position_numbers(
        &position_to_close,
        &token_trade_price,
        &collateral_token_price,
        &collateral_custody,
        current_time,
        false,
    )?;

    if exit_numbers.deficit_fee_usd > 0 || exit_numbers.deficit_pool_usd > 0 {
        msg!("====== deficit_fee_usd: {}", exit_numbers.deficit_fee_usd);
        msg!("====== deficit_pool_usd: {}", exit_numbers.deficit_pool_usd);
    }

    // Unlock tokens from collateral custody
    collateral_custody.unlock_funds(position_to_close.locked_amount)?;

    // Check there are enough tokens available to pay the user with
    require!(
        pool.check_available_amount(exit_numbers.close_amount, &collateral_custody)?,
        AdrenaError::CustodyAmountLimit
    );

    // Pay user profits
    if exit_numbers.close_amount > 0 {
        cortex.transfer_tokens(
            ctx.accounts
                .collateral_custody_token_account
                .to_account_info(),
            ctx.accounts.receiving_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            exit_numbers.close_amount,
        )?;
    }

    // Update custody stats
    {
        collateral_custody.collected_fees.close_position_usd += exit_numbers.exit_fee_usd;

        // Account for borrow fee (do not count the paid interest as it has already been accounted for)
        collateral_custody.collected_fees.borrow_usd += exit_numbers
            .borrow_fee_usd
            .saturating_sub(position_to_close.paid_interest_usd);

        // Account for user profit/loss on collateral assets
        {
            if exit_numbers.close_amount > position_to_close.collateral_amount {
                let amount_lost = exit_numbers
                    .close_amount
                    .saturating_sub(position_to_close.collateral_amount);

                msg!("[Custody] Amount lost: {}", amount_lost);

                collateral_custody.assets.owned -= amount_lost;
            } else {
                let amount_gained = position_to_close
                    .collateral_amount
                    .saturating_sub(exit_numbers.close_amount);

                msg!("[Custody] Amount gained: {}", amount_gained);

                collateral_custody.assets.owned += amount_gained;
            }
        }

        collateral_custody.assets.collateral -= position_to_close.collateral_amount;

        custody.volume_stats.close_position_usd += position_to_close.size_usd;

        // Avoid double counting: the paid interest is already accounted for in the stats
        custody.trade_stats.loss_usd += exit_numbers
            .loss_usd
            .saturating_sub(position_to_close.paid_interest_usd);

        custody.trade_stats.profit_usd += exit_numbers.profit_usd;
        custody.trade_stats.oi_short_usd -= position_to_close.size_usd;

        // If it's a partial close, trick it by doing +1 to nullify the -1 in update_accounting_after_remove_position_long
        if is_partial_close {
            custody.short_positions.open_positions += 1;
            collateral_custody.short_positions.open_positions += 1;
        }

        custody.update_accounting_after_remove_position_short(
            &position_to_close,
            current_time,
            &mut collateral_custody,
        )?;
    }

    let fee_usd_to_distribute = exit_numbers
        .total_fee_usd
        .saturating_sub(position_to_close.paid_interest_usd);

    // Update fee debt accounting + referral
    {
        let referrer_fee_usd = if has_referrer {
            cortex.get_protocol_fee(fee_usd_to_distribute)?
        } else {
            0
        };

        // NOTE: Do not count LP fee as part of the fee debt as the fee stay in the pool
        {
            let lp_fee_usd = cortex.get_lp_fee(fee_usd_to_distribute)?;
            let fee_usd = fee_usd_to_distribute - referrer_fee_usd - lp_fee_usd;

            pool.fees_debt_usd += fee_usd;
        }

        pool.referrers_fee_debt_usd += referrer_fee_usd;
        pool.cumulative_referrer_fee_usd += referrer_fee_usd;

        if has_referrer {
            let mut referrer_profile =
                ctx.accounts.referrer_profile.as_ref().unwrap().load_mut()?;

            referrer_profile.claimable_referral_fee_usd += referrer_fee_usd;
            referrer_profile.total_referral_fee_usd += referrer_fee_usd;
        }
    }

    // Update borrow rate (because we added/removed tokens as "owned" in the custody)
    {
        collateral_custody.update_borrow_rate(current_time)?;
    }

    if is_partial_close {
        // Remove what's already been closed from the position
        position.subtract(&position_to_close)?;
    } else {
        // Delete the position
        Cortex::transfer_sol_from_owned(
            ctx.accounts.position.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.position.get_lamports(),
        )?;

        // Deallocate the position account
        *position = Position::default();
    }

    // Ensure the position is not too small after partial close
    if is_partial_close {
        require!(
            position.collateral_usd >= Cortex::POSITION_MIN_COLLATERAL_VALUE,
            AdrenaError::InsufficientCollateral
        );
    }

    emit!(ClosePositionEvent {
        owner: ctx.accounts.owner.key(),
        position: ctx.accounts.position.key(),
        custody_mint,
        side: position_to_close.side,
        size_usd: position_to_close.size_usd,
        price: token_trade_price.price,
        collateral_amount_usd: position_to_close.collateral_usd,
        profit_usd: exit_numbers.profit_usd,
        loss_usd: exit_numbers.loss_usd,
        borrow_fee_usd: exit_numbers.borrow_fee_usd,
        exit_fee_usd: exit_numbers.exit_fee_usd,
        position_id: position_to_close.id,
        percentage: params.percentage,
    });

    Ok(())
}
