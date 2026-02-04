use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::{Cortex, NewPositionPricesAndFee},
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
            position::{Position, Side},
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct GetEntryPriceAndFee<'info> {
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
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #4
    #[account(
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #5
    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 collateral_custody.load()?.mint.as_ref()],
        bump = collateral_custody.load()?.bump
    )]
    pub collateral_custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetEntryPriceAndFeeParams {
    pub collateral: u64,
    pub leverage: u32,
    pub side: u8, // Side
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_entry_price_and_fee(
    ctx: Context<GetEntryPriceAndFee>,
    params: &GetEntryPriceAndFeeParams,
) -> Result<NewPositionPricesAndFee> {
    let side = Side::try_from(params.side)?;

    let oracle = ctx.accounts.oracle.load()?;

    // Preliminary checks
    {
        if params.collateral == 0 || params.leverage == 0 || side == Side::None {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    let pool = *ctx.accounts.pool.load()?;
    let custody = *ctx.accounts.custody.load()?;
    let collateral_custody = *ctx.accounts.collateral_custody.load()?;

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

    // Calculate amounts
    #[allow(clippy::type_complexity)]
    let (
        size,
        size_usd,
        collateral_usd,
        exit_fee,
        exit_fee_usd,
        liquidation_fee,
        liquidation_fee_usd,
    ) = (|| -> Result<(u64, u64, u64, u64, u64, u64, u64)> {
        let collateral = params.collateral;
        let collateral_usd = collateral_token_price
            .low()
            .get_asset_amount_usd(collateral, collateral_custody.decimals)?;

        // In collateral
        let size =
            math::checked_as_u64(collateral as u128 * params.leverage as u128 / Cortex::BPS_POWER)?;

        let size_usd = collateral_token_price
            .low()
            .get_asset_amount_usd(size, collateral_custody.decimals)?;

        // Calculate OUT fees (unrealized, estimated)
        let (exit_fee, exit_fee_usd) = {
            // In collateral
            let exit_fee = pool.get_exit_fee(size, &custody)?;

            (
                exit_fee,
                collateral_token_price
                    .get_asset_amount_usd(exit_fee, collateral_custody.decimals)?,
            )
        };

        // Estimations
        let (liquidation_fee, liquidation_fee_usd) = {
            let liquidation_fee: u64 = pool.get_liquidation_fee(size, &custody)?;

            (
                liquidation_fee,
                collateral_token_price
                    .get_asset_amount_usd(liquidation_fee, collateral_custody.decimals)?,
            )
        };

        Ok((
            size,
            size_usd,
            collateral_usd,
            exit_fee,
            exit_fee_usd,
            liquidation_fee,
            liquidation_fee_usd,
        ))
    })()?;

    let entry_price = match side {
        Side::Long => token_trade_price.price,
        Side::Short => token_trade_price.price,
        Side::None => return Err(ProgramError::InvalidArgument.into()),
    };
    let position = Position {
        side: side.into(),
        price: entry_price,
        size_usd,
        collateral_usd,
        cumulative_interest_snapshot: U128Split::from(
            collateral_custody.get_cumulative_interest(current_time)?,
        ),
        exit_fee_usd,
        liquidation_fee_usd,
        ..Position::default()
    };

    // Based on estimated without the real confidence at close
    let liquidation_price =
        pool.get_liquidation_price(&position, &custody, &collateral_custody, current_time)?;

    Ok(NewPositionPricesAndFee {
        entry_price,
        liquidation_price,
        exit_fee,
        liquidation_fee,
        size,
    })
}
