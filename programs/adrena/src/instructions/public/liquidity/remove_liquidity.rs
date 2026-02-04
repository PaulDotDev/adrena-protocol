use {
    crate::{
        error::AdrenaError,
        math,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    num::Zero,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2 Front end will target the owner account, but not limited to
    #[account(
        mut,
        constraint = receiving_account.mint == custody.load()?.mint,
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// #3
    #[account(
        mut,
        constraint = lp_token_account.mint == lp_token_mint.key(),
        has_one = owner
    )]
    pub lp_token_account: Box<Account<'info, TokenAccount>>,

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
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
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
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #11
    pub token_program: Program<'info, Token>,

    /// #12
    pub adrena_program: Program<'info, Adrena>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RemoveLiquidityParams {
    pub lp_amount_in: u64,
    pub min_amount_out: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn remove_liquidity(
    ctx: Context<RemoveLiquidity>,
    params: &RemoveLiquidityParams,
) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = cortex.get_time()?;

    // Preliminary checks
    {
        if params.lp_amount_in == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

    // In order to protect the protocol from volatility, we apply a confidence
    let token_price_low = token_price.low();
    msg!(
        "Current price: {} (low {})",
        token_price.price,
        token_price_low.price
    );

    // Refresh pool.aum_usm to adapt to token price change
    {
        // Must drop to let get assets under management load remaining accounts about the same custody
        drop(custody);

        // Refresh pool.aum_usm to adapt to token price change
        pool.aum_usd = U128Split::new(pool.get_assets_under_management_usd(
            &oracle,
            ctx.remaining_accounts,
            current_time,
        )?);

        custody = ctx.accounts.custody.load_mut()?;
    }

    // compute amount of tokens to return
    let remove_amount_usd = math::checked_as_u64(
        (pool.aum_usd.to_u128() * params.lp_amount_in as u128)
            / ctx.accounts.lp_token_mint.supply as u128,
    )?;

    let remove_amount = token_price_low.get_token_amount(remove_amount_usd, custody.decimals)?;

    // calculate fee
    let fee_amount = pool.get_remove_liquidity_fee(remove_amount, &custody)?;

    msg!("Collected fee: {}", fee_amount);

    let transfer_amount = remove_amount - fee_amount;

    msg!("Amount out: {}", transfer_amount);
    msg!("LP amount in: {}", params.lp_amount_in);

    require!(
        transfer_amount >= params.min_amount_out,
        AdrenaError::MaxPriceSlippage
    );

    let withdrawal_amount = transfer_amount;

    cortex.transfer_tokens(
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        transfer_amount,
    )?;

    // Burn LP tokens
    {
        cortex.burn_tokens_from_user(
            ctx.accounts.lp_token_mint.to_account_info(),
            ctx.accounts.lp_token_account.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.lp_amount_in,
        )?;

        // Make sure the lp token mint is updated before calculating the fee distribution
        ctx.accounts.lp_token_mint.reload()?;
    }

    // Update custody stats
    {
        let fee_usd = token_price.get_asset_amount_usd(fee_amount, custody.decimals)?;

        custody.collected_fees.remove_liquidity_usd = custody
            .collected_fees
            .remove_liquidity_usd
            .wrapping_add(fee_usd);

        custody.volume_stats.remove_liquidity_usd = custody
            .volume_stats
            .remove_liquidity_usd
            .wrapping_add(remove_amount_usd);

        custody.assets.owned -= withdrawal_amount;
    }

    // Update fee debt accounting
    {
        // NOTE: Do not count LP fee as part of the fee debt as the fee stay in the pool
        let fee_amount_less_lp_fee = fee_amount - cortex.get_lp_fee(fee_amount)?;

        let fee_usd = token_price.get_asset_amount_usd(fee_amount_less_lp_fee, custody.decimals)?;

        pool.fees_debt_usd += fee_usd;
    }

    // Update pool AUM and borrow rate (because we removed tokens from "owned" in the custody)
    {
        // Must drop to let get assets under management load remaining accounts about the same custody
        drop(custody);

        let aum_usd =
            pool.get_assets_under_management_usd(&oracle, ctx.remaining_accounts, current_time)?;

        pool.aum_usd = U128Split::new(aum_usd);

        msg!("Pool AUM (usd): {}", pool.aum_usd.to_u128());

        // Update LP token price
        let lp_supply = ctx.accounts.lp_token_mint.supply;

        pool.lp_token_price_usd = if lp_supply.is_zero() {
            0
        } else {
            math::checked_decimal_div(
                math::checked_as_u64(aum_usd)?,
                -(Cortex::USD_DECIMALS as i32),
                lp_supply,
                -(Cortex::LP_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?
        };

        pool.last_aum_and_lp_token_price_usd_update = current_time;

        custody = ctx.accounts.custody.load_mut()?;

        // Update borrow rate post fees distribution
        custody.update_borrow_rate(current_time)?;

        if custody.is_stable() {
            require!(
                custody.assets.owned as u128 >= 200_000u128 * 10u128.pow(custody.decimals as u32),
                AdrenaError::CustodyBelowMinimum
            );
        }
    }

    Ok(())
}
