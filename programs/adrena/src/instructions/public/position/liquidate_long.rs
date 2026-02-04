use {
    crate::{
        error::AdrenaError,
        events::LiquidateEvent,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{LeverageCheckType, Pool},
            position::{Position, Side},
            user_profile::UserProfile,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct LiquidateLong<'info> {
    /// #1
    #[account(mut)]
    pub signer: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = receiving_account.mint == custody.load()?.mint,
        constraint = receiving_account.owner == position.load()?.owner
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
                 position.load()?.owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        bump = position.load()?.bump,
        close = signer,
        constraint = position.load()?.get_side() == Side::Long,
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
    #[account(
        mut,
        seeds = [b"user_profile",
                 position.load()?.owner.as_ref()],
        bump = user_profile.load()?.bump
    )]
    pub user_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #11
    #[account(mut)]
    pub referrer_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #12
    pub token_program: Program<'info, Token>,

    /// #13
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LiquidateLongParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn liquidate_long(ctx: Context<LiquidateLong>, params: &LiquidateLongParams) -> Result<()> {
    let cortex: std::cell::RefMut<'_, Cortex> = ctx.accounts.cortex.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let position = ctx.accounts.position.load_mut()?;
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Preliminary checks
    let has_referrer = {
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

    let current_time = cortex.get_time()?;

    // TODO
    // Load the oracle prices, while enforcing pixel perfect mechanism:
    //
    // The instruction can be called with an old price, where the position was eligible for liquidation.
    // This mechanism allows to alleviate the pain of congestion and transaction landing.
    // If the position wasn't liquidated timely, we will still liquidate it at the right price even minutes later.
    // Benefitting the trader, while not hurting the protocol.
    let (token_trade_price, collateral_token_price) = {
        let mut oracle = ctx.accounts.oracle.load_mut()?;

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }

        let token_trade_price = oracle.get_oracle_price(custody.trade_oracle, current_time)?;
        let collateral_token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

        (token_trade_price, collateral_token_price)
    };

    msg!(
        "Collateral price: {} (high {}, low {})",
        collateral_token_price.price,
        collateral_token_price.high().price,
        collateral_token_price.low().price
    );
    msg!("Trade price: {}", token_trade_price.price);

    // check if position can be liquidated
    {
        // Position needs to be above max leverage
        let leverage = pool.check_leverage(
            &position,
            &token_trade_price,
            &custody,
            &collateral_token_price,
            &custody,
            current_time,
            LeverageCheckType::Liquidate,
        );

        match leverage {
            Ok(_) => return Err(AdrenaError::PositionNotInLiquidationRange.into()),
            Err(e) => require!(
                e == AdrenaError::MaxLeverage.into(),
                AdrenaError::PositionNotInLiquidationRange
            ),
        };
    }

    let exit_numbers = pool.get_exit_position_numbers(
        &position,
        &token_trade_price,
        &collateral_token_price,
        &custody,
        current_time,
        true,
    )?;

    if exit_numbers.deficit_fee_usd > 0 || exit_numbers.deficit_pool_usd > 0 {
        msg!("====== deficit_fee_usd: {}", exit_numbers.deficit_fee_usd);
        msg!("====== deficit_pool_usd: {}", exit_numbers.deficit_pool_usd);
    }

    // Unlock tokens from custody
    custody.unlock_funds(position.locked_amount)?;

    require!(
        pool.check_available_amount(exit_numbers.close_amount, &custody)?,
        AdrenaError::CustodyAmountLimit
    );

    if exit_numbers.close_amount > 0 {
        msg!(
            "Transfer collateral tokens to user: {}",
            exit_numbers.close_amount
        );

        cortex.transfer_tokens(
            ctx.accounts.custody_token_account.to_account_info(),
            ctx.accounts.receiving_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            exit_numbers.close_amount,
        )?;
    }

    // Update custodies stats
    {
        custody.collected_fees.liquidation_usd += exit_numbers.exit_fee_usd;

        // Account for borrow fee (do not count the paid interest as it has already been accounted for)
        custody.collected_fees.borrow_usd += exit_numbers
            .borrow_fee_usd
            .saturating_sub(position.paid_interest_usd);

        // Account for user profit/loss on collateral assets
        {
            if exit_numbers.close_amount > position.collateral_amount {
                let amount_lost = exit_numbers
                    .close_amount
                    .saturating_sub(position.collateral_amount);

                msg!("amount_lost: {}", amount_lost);

                custody.assets.owned -= amount_lost;
            } else {
                let amount_gained = position
                    .collateral_amount
                    .saturating_sub(exit_numbers.close_amount);

                msg!("amount_gained: {}", amount_gained);

                custody.assets.owned += amount_gained;
            }
        }

        custody.assets.collateral -= position.collateral_amount;

        custody.volume_stats.liquidation_usd += position.size_usd;

        custody.trade_stats.oi_long_usd -= position.size_usd;

        // Avoid double counting: the paid interest is already accounted for in the stats
        custody.trade_stats.loss_usd += exit_numbers
            .loss_usd
            .saturating_sub(position.paid_interest_usd);

        custody.trade_stats.profit_usd += exit_numbers.profit_usd;

        custody.update_accounting_after_remove_position_long(&position, current_time)?;
    }

    let fee_usd_to_distribute = exit_numbers
        .total_fee_usd
        .saturating_sub(position.paid_interest_usd);

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
        custody.update_borrow_rate(current_time)?;
    }

    emit!(LiquidateEvent {
        owner: position.owner,
        position: ctx.accounts.position.key(),
        custody_mint: custody.mint.key(),
        side: position.side,
        size_usd: position.size_usd,
        price: token_trade_price.price,
        collateral_amount_usd: position.collateral_usd,
        loss_usd: exit_numbers.loss_usd,
        borrow_fee_usd: exit_numbers.borrow_fee_usd,
        exit_fee_usd: exit_numbers.exit_fee_usd,
        position_id: position.id,
    });

    Ok(())
}
