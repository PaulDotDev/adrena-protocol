use {
    crate::{
        error::AdrenaError,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool, position::Position,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetLiquidationPrice<'info> {
    /// #1
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #2
    #[account(
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #3
    #[account(
        seeds = [b"position",
                 position.load()?.owner.as_ref(),
                 pool.key().as_ref(),
                 custody.key().as_ref(),
                 &[position.load()?.side]],
        bump = position.load()?.bump
    )]
    pub position: AccountLoader<'info, Position>,

    /// #4
    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #5
    #[account(
        constraint = position.load()?.collateral_custody == collateral_custody.key()
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,

    /// #6
    #[account(
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetLiquidationPriceParams {
    pub add_collateral: u64,
    pub remove_collateral: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_liquidation_price(
    ctx: Context<GetLiquidationPrice>,
    params: &GetLiquidationPriceParams,
) -> Result<u64> {
    let pool = ctx.accounts.pool.load()?;
    let custody = ctx.accounts.custody.load()?;
    let collateral_custody = &ctx.accounts.collateral_custody.load()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;
    let oracle = ctx.accounts.oracle.load()?;

    // Load the oracle prices
    let collateral_token_price = {
        match &params.oracle_prices {
            Some(p) => {
                p.verify_signature()?;

                oracle.get_up_to_date_prices_readonly(
                    p,
                    vec![collateral_custody.oracle],
                    current_time,
                )?[0]
            }
            None => oracle.get_oracle_price(collateral_custody.oracle, current_time)?,
        }
    };

    let current_time = Clock::get()?.unix_timestamp;

    // only mutated locally
    #[allow(clippy::clone_on_copy)]
    let mut position = ctx.accounts.position.load()?.clone();

    position.update_time = current_time;

    if params.add_collateral > 0 {
        let collateral_usd = collateral_token_price
            .low()
            .get_asset_amount_usd(params.add_collateral, collateral_custody.decimals)?;

        position.collateral_usd += collateral_usd;
        position.collateral_amount += params.add_collateral;
    }

    if params.remove_collateral > 0 {
        let collateral_usd = collateral_token_price
            .high()
            .get_asset_amount_usd(params.remove_collateral, collateral_custody.decimals)?;

        if collateral_usd >= position.collateral_usd
            || params.remove_collateral >= position.collateral_amount
        {
            return Err(ProgramError::InsufficientFunds.into());
        }

        position.collateral_usd -= collateral_usd;
        position.collateral_amount -= params.remove_collateral;
    }

    let liquidation_price =
        pool.get_liquidation_price(&position, &custody, collateral_custody, current_time)?;

    Ok(liquidation_price)
}
