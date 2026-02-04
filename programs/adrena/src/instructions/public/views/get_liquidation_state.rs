use {
    crate::{
        error::AdrenaError,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{LeverageCheckType, Pool},
            position::Position,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetLiquidationState<'info> {
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
        constraint = position.load()?.collateral_custody == collateral_custody.key()
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetLiquidationStateParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_liquidation_state(
    ctx: Context<GetLiquidationState>,
    params: &GetLiquidationStateParams,
) -> Result<u8> {
    let pool = ctx.accounts.pool.load()?;
    let custody = ctx.accounts.custody.load()?;
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

    let position = ctx.accounts.position.load()?;

    let leverage = pool.check_leverage(
        &position,
        &token_trade_price,
        &custody,
        &collateral_token_price,
        collateral_custody,
        current_time,
        LeverageCheckType::Liquidate,
    );

    match leverage {
        Ok(_) => Ok(0),
        Err(e) => {
            if e == AdrenaError::MaxLeverage.into() {
                Ok(1)
            } else {
                Err(e)
            }
        }
    }
}
