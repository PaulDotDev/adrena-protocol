use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, oracle::Oracle, pool::Pool,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Mint,
    num::Zero,
};

#[derive(Accounts)]
pub struct UpdatePoolAum<'info> {
    /// #1
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #2
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #3
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #4
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #5
    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdatePoolAumParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn update_pool_aum(ctx: Context<UpdatePoolAum>, params: &UpdatePoolAumParams) -> Result<u128> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    let current_time: i64 = ctx.accounts.cortex.load()?.get_time()?;

    let mut oracle = ctx.accounts.oracle.load_mut()?;

    // Preliminary checks
    {
        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    // Update AUM and LP token price
    {
        let aum_usd =
            pool.get_assets_under_management_usd(&oracle, ctx.remaining_accounts, current_time)?;

        pool.aum_usd = U128Split::new(aum_usd);

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
    }

    Ok(pool.aum_usd.to_u128())
}
