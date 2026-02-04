use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool,
        },
    },
    anchor_lang::{self, prelude::*},
    anchor_spl::token::{Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct Swap<'info> {
    /// #1
    #[account()]
    pub caller: Signer<'info>,

    /// #2
    #[account()]
    pub owner: Signer<'info>,

    /// #3
    #[account(
        mut,
        constraint = funding_account.mint == receiving_custody.load()?.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #4
    #[account(
        mut,
        constraint = receiving_account.mint == dispensing_custody.load()?.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

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
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
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
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 receiving_custody.load()?.mint.as_ref()],
        bump = receiving_custody.load()?.bump
    )]
    pub receiving_custody: AccountLoader<'info, Custody>,

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
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 receiving_custody.load()?.mint.as_ref()],
        bump = receiving_custody.load()?.token_account_bump
    )]
    pub receiving_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #11
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 dispensing_custody.load()?.mint.as_ref()],
        bump = dispensing_custody.load()?.bump
    )]
    pub dispensing_custody: AccountLoader<'info, Custody>,

    /// #12
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 dispensing_custody.load()?.mint.as_ref()],
        bump = dispensing_custody.load()?.token_account_bump
    )]
    pub dispensing_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #13
    pub token_program: Program<'info, Token>,

    /// #14
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapParams {
    pub amount_in: u64,
    pub min_amount_out: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn swap(ctx: Context<Swap>, params: &SwapParams) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let mut receiving_custody = ctx.accounts.receiving_custody.load_mut()?;
    let mut dispensing_custody = ctx.accounts.dispensing_custody.load_mut()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    let receiving_custody_key = ctx.accounts.receiving_custody.key();
    let dispensing_custody_key = ctx.accounts.dispensing_custody.key();
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = cortex.get_time()?;

    // Preliminary checks
    {
        require!(
            pool.is_swap_allowed()
                && receiving_custody.allow_swap()
                && dispensing_custody.allow_swap(),
            AdrenaError::InstructionNotAllowed
        );

        if params.amount_in == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        require_keys_neq!(receiving_custody_key, dispensing_custody_key);

        // Call to swap must be either internal, or the owner must be whitelisted
        require!(
            ctx.accounts.caller.key() == ctx.accounts.transfer_authority.key()
                || (ctx.accounts.caller.key() == pool.whitelisted_swapper
                    && ctx.accounts.caller.key() == ctx.accounts.owner.key()),
            AdrenaError::InstructionNotAllowed
        );

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    msg!("Token in: {}", params.amount_in);

    let current_time = cortex.get_time()?;

    let received_token_price = oracle.get_oracle_price(receiving_custody.oracle, current_time)?;

    let received_token_price_low = if ctx.accounts.caller.key() == pool.whitelisted_swapper
        && ctx.accounts.caller.key() == ctx.accounts.owner.key()
    {
        received_token_price
    } else {
        received_token_price.low()
    };
    msg!(
        "Current received_token price: {} (low {})",
        received_token_price.price,
        received_token_price_low.price
    );

    let dispensed_token_price = oracle.get_oracle_price(dispensing_custody.oracle, current_time)?;

    let (dispensed_token_price_high, dispensed_token_price_low) = if ctx.accounts.caller.key()
        == pool.whitelisted_swapper
        && ctx.accounts.caller.key() == ctx.accounts.owner.key()
    {
        (dispensed_token_price, dispensed_token_price)
    } else {
        (dispensed_token_price.high(), dispensed_token_price.low())
    };

    msg!(
        "Current dispensed_token price: {} (high {}, low {})",
        dispensed_token_price.price,
        dispensed_token_price_high.price,
        dispensed_token_price_low.price
    );

    // Internal swaps are not subject to fees.
    // Same when the caller is the whitelisted swapper
    //
    // Internal swaps are used to convert protocol collected fee back to stable for staking rewards.
    // Whitelisted swapper is used for rebalancing the pool with reasonable cost.
    let feeless_swap = ctx.accounts.owner.key() == ctx.accounts.transfer_authority.key()
        || ctx.accounts.caller.key() == pool.whitelisted_swapper;

    // Calculate fee in
    // when it's an internal swap, no fees are taken
    let fees_in_amount = match feeless_swap {
        true => 0,
        false => {
            pool.get_swap_in_fees(params.amount_in, &receiving_custody, &dispensing_custody)?
        }
    };

    let amount_out = pool.get_swap_amount(
        &received_token_price_low,
        &dispensed_token_price_high,
        &receiving_custody,
        &dispensing_custody,
        params.amount_in - fees_in_amount,
    )?;

    // Calculate fee out
    // when it's an internal swap, no fees are taken
    let fees_out_amount = match feeless_swap {
        true => 0,
        false => pool.get_swap_out_fees(amount_out, &receiving_custody, &dispensing_custody)?,
    };

    msg!("Collected fees: {} {}", fees_in_amount, fees_out_amount);

    // Check returned amount
    let no_fee_amount = amount_out - fees_out_amount;

    msg!("Amount out: {}", amount_out);

    require_gte!(
        no_fee_amount,
        params.min_amount_out,
        AdrenaError::InsufficientAmountReturned
    );

    let deposit_amount = params.amount_in;
    let withdrawal_amount = no_fee_amount;

    // Transfer tokens
    {
        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts
                .receiving_custody_token_account
                .to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.amount_in,
        )?;

        msg!("Token out: {}", no_fee_amount);

        cortex.transfer_tokens(
            ctx.accounts
                .dispensing_custody_token_account
                .to_account_info(),
            ctx.accounts.receiving_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            no_fee_amount,
        )?;
    }

    // Update custody stats
    {
        let fees_in_usd = received_token_price
            .get_asset_amount_usd(fees_in_amount, receiving_custody.decimals)?;

        let fees_out_usd = dispensed_token_price
            .get_asset_amount_usd(fees_out_amount, dispensing_custody.decimals)?;

        receiving_custody.volume_stats.swap_usd =
            receiving_custody.volume_stats.swap_usd.wrapping_add(
                received_token_price_low
                    .get_asset_amount_usd(params.amount_in, receiving_custody.decimals)?,
            );

        receiving_custody.collected_fees.swap_usd = receiving_custody
            .collected_fees
            .swap_usd
            .wrapping_add(fees_in_usd);

        // Account for all fees there as part of the custody. distribute_fees will take care of removing it from the custody
        receiving_custody.assets.owned += deposit_amount;

        dispensing_custody.collected_fees.swap_usd = dispensing_custody
            .collected_fees
            .swap_usd
            .wrapping_add(fees_out_usd);

        dispensing_custody.volume_stats.swap_usd =
            dispensing_custody.volume_stats.swap_usd.wrapping_add(
                dispensed_token_price_low
                    .get_asset_amount_usd(amount_out, dispensing_custody.decimals)?,
            );

        dispensing_custody.assets.owned -= withdrawal_amount;
    }

    // Swap the collected fee_amount to stable and send to staking rewards
    // when it's an internal swap, no fees swap is done
    if feeless_swap {
        return Ok(());
    }

    // Update fee debt accounting
    {
        // NOTE: Do not count LP fee as part of the fee debt as the fee stay in the pool
        let fee_in_amount_less_lp_fee = fees_in_amount - cortex.get_lp_fee(fees_in_amount)?;

        let fee_out_amount_less_lp_fee = fees_out_amount - cortex.get_lp_fee(fees_out_amount)?;

        let fees_in_usd = received_token_price
            .get_asset_amount_usd(fee_in_amount_less_lp_fee, receiving_custody.decimals)?;

        let fees_out_usd = dispensed_token_price
            .get_asset_amount_usd(fee_out_amount_less_lp_fee, dispensing_custody.decimals)?;

        let fee_usd = fees_in_usd + fees_out_usd;

        pool.fees_debt_usd += fee_usd;
    }

    // Update borrow rate (because we added/removed tokens as "owned" in the custodies)
    {
        receiving_custody.update_borrow_rate(current_time)?;
        dispensing_custody.update_borrow_rate(current_time)?;
    }

    Ok(())
}
