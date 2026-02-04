use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, oracle::Oracle, pool::Pool,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Mint,
    num_traits::Zero,
};

#[derive(Accounts)]
pub struct GetLpTokenPrice<'info> {
    /// #1
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #2
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #3
    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #4
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

// This is a semi-view function. Having oracle_prices mutable for code simplicity
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetLpTokenPriceParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_lp_token_price(
    ctx: Context<GetLpTokenPrice>,
    params: &GetLpTokenPriceParams,
) -> Result<u64> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Preliminary checks
    {
        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let aum_usd = math::checked_as_u64(pool.get_assets_under_management_usd(
        &oracle,
        ctx.remaining_accounts,
        ctx.accounts.cortex.load()?.get_time()?,
    )?)?;

    msg!("Pool AUM (usd): {}", aum_usd);

    let lp_supply = ctx.accounts.lp_token_mint.supply;

    msg!("LP supply: {}", lp_supply);

    if lp_supply.is_zero() {
        return Ok(0);
    }

    let price_usd = math::checked_decimal_div(
        aum_usd,
        -(Cortex::USD_DECIMALS as i32),
        lp_supply,
        -(Cortex::LP_DECIMALS as i32),
        -(Cortex::PRICE_DECIMALS as i32),
    )?;

    msg!("Price (usd): {}", price_usd);

    pool.lp_token_price_usd = price_usd;

    pool.last_aum_and_lp_token_price_usd_update = current_time;

    Ok(price_usd)
}
