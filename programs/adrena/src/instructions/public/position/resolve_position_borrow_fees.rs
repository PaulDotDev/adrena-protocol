use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool, position::Position, user_profile::UserProfile,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
};

#[derive(Accounts)]
pub struct ResolvePositionBorrowFees<'info> {
    /// #1
    #[account(mut)]
    pub signer: Signer<'info>,

    /// #2
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #3
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #5
    #[account(
        mut,
        seeds = [b"position",
                 position.load()?.owner.key().as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        has_one = custody,
        has_one = collateral_custody,
        has_one = pool,
        bump = position.load()?.bump,
    )]
    pub position: AccountLoader<'info, Position>,

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
        constraint = position.load()?.custody == custody.key()
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #8
    #[account(
        mut,
        constraint = position.load()?.collateral_custody == collateral_custody.key()
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        seeds = [b"user_profile",
                 position.load()?.owner.as_ref()],
        bump = user_profile.load()?.bump
    )]
    pub user_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #10
    #[account(mut)]
    pub referrer_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #11
    pub token_program: Program<'info, Token>,

    /// #12
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ResolvePositionBorrowFeesParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn resolve_position_borrow_fees(
    ctx: Context<ResolvePositionBorrowFees>,
    params: &ResolvePositionBorrowFeesParams,
) -> Result<()> {
    let mut position = ctx.accounts.position.load_mut()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    let cortex = ctx.accounts.cortex.load()?;

    let current_time = cortex.get_time()?;

    // Preliminary checks
    let has_referrer = {
        let mut oracle = ctx.accounts.oracle.load_mut()?;

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
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

    let mut collateral_custody = ctx.accounts.collateral_custody.load_mut()?;

    let interest_usd_due = collateral_custody.get_interest_amount_usd(&position, current_time)?
        + position.unrealized_interest_usd;

    msg!("Interest due: {}", interest_usd_due);

    if interest_usd_due == 0 {
        return Ok(());
    }

    // Update position to account for paid interest
    {
        // Consider it paid now
        position.unrealized_interest_usd = 0;

        // Reset the borrow fees counter so it starts from 0 again
        position.cumulative_interest_snapshot =
            U128Split::from(collateral_custody.get_cumulative_interest(current_time)?);

        position.paid_interest_usd += interest_usd_due;
    }

    // Update fee debt accounting + referral
    {
        let referrer_fee_usd = if has_referrer {
            cortex.get_protocol_fee(interest_usd_due)?
        } else {
            0
        };

        // NOTE: Do not count LP fee as part of the fee debt as the fee stay in the pool
        {
            let lp_fee_usd = cortex.get_lp_fee(interest_usd_due)?;
            let fee_usd = interest_usd_due - referrer_fee_usd - lp_fee_usd;

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

    // Update custody stats
    {
        collateral_custody.collected_fees.borrow_usd += interest_usd_due;

        if ctx.accounts.custody.key() == ctx.accounts.collateral_custody.key() {
            collateral_custody.trade_stats.loss_usd += interest_usd_due;
        } else {
            ctx.accounts.custody.load_mut()?.trade_stats.loss_usd += interest_usd_due;
        }
    }

    Ok(())
}
