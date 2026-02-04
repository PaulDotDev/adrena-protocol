use {
    crate::{
        error::AdrenaError,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::{Cortex, ExitPriceAndFee},
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
            position::{Position, Side},
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetExitPriceAndFee<'info> {
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
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #6
    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.bump
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetExitPriceAndFeeParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

// This is an approximation of the exit price and fee.
// It partially accounts for the interest that accrues over time due to the Borrow Rate, although only take it up to the last resolution
pub fn get_exit_price_and_fee(
    ctx: Context<GetExitPriceAndFee>,
    params: &GetExitPriceAndFeeParams,
) -> Result<ExitPriceAndFee> {
    // compute exit price and fee
    let position = &ctx.accounts.position.load()?;
    let custody = &ctx.accounts.custody.load()?;
    let collateral_custody = &ctx.accounts.collateral_custody.load()?;
    let oracle = ctx.accounts.oracle.load()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Load the oracle prices
    let (token_trade_price, collateral_token_price) = {
        match &params.oracle_prices {
            Some(p) => {
                p.verify_signature()?;

                let ret = oracle.get_up_to_date_prices_readonly(
                    p,
                    vec![custody.trade_oracle, collateral_custody.oracle],
                    current_time,
                )?;

                (ret[0], ret[1])
            }
            None => (
                oracle.get_oracle_price(custody.trade_oracle, current_time)?,
                oracle.get_oracle_price(collateral_custody.oracle, current_time)?,
            ),
        }
    };

    let unrealized_interest_usd = custody.get_interest_amount_usd(position, current_time)?;
    let position_exit_fee_usd =
        position.exit_fee_usd + unrealized_interest_usd + position.unrealized_interest_usd;

    let fee = collateral_token_price
        .high()
        .get_token_amount(position_exit_fee_usd, collateral_custody.decimals)?;

    let exit_numbers = ctx.accounts.pool.load()?.get_exit_position_numbers(
        position,
        &token_trade_price,
        &collateral_token_price,
        collateral_custody,
        current_time,
        false,
    )?;

    let exit_price = match Side::try_from(position.side)? {
        Side::Long => token_trade_price.price,
        Side::Short => token_trade_price.price,
        Side::None => return Err(AdrenaError::InvalidAccountData.into()),
    };
    Ok(ExitPriceAndFee {
        price: exit_price,
        fee,
        amount_out: exit_numbers.close_amount,
        profit_usd: exit_numbers.profit_usd,
        loss_usd: exit_numbers.loss_usd,
    })
}
