use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::{AmountAndFee, Cortex},
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Mint,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct GetAddLiquidityAmountAndFee<'info> {
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
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetAddLiquidityAmountAndFeeParams {
    pub amount_in: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_add_liquidity_amount_and_fee(
    ctx: Context<GetAddLiquidityAmountAndFee>,
    params: &GetAddLiquidityAmountAndFeeParams,
) -> Result<AmountAndFee> {
    let oracle = ctx.accounts.oracle.load()?;

    // validate inputs
    if params.amount_in == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }

    let pool = ctx.accounts.pool.load()?;
    let custody = *ctx.accounts.custody.load()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Load the oracle prices
    let token_price = {
        match &params.oracle_prices {
            Some(p) => {
                p.verify_signature()?;

                oracle.get_up_to_date_prices_readonly(p, vec![custody.oracle], current_time)?[0]
            }
            None => oracle.get_oracle_price(custody.oracle, current_time)?,
        }
    };

    // In order to protect the protocol from volatility, we apply a confidence
    let token_price_high = token_price.high();
    let token_price_low = token_price.low();
    msg!(
        "Current price: {} (high {}, low {})",
        token_price.price,
        token_price_high.price,
        token_price_low.price
    );

    let fee_amount = pool.get_add_liquidity_fee(params.amount_in, &custody)?;

    msg!("Params amount in: {}", params.amount_in);
    msg!("Fee amount: {}", fee_amount);

    let no_fee_amount = params.amount_in - fee_amount;
    msg!("No fee amount: {}", no_fee_amount);

    let token_amount_usd = token_price_low.get_asset_amount_usd(no_fee_amount, custody.decimals)?;
    msg!("Token amount (usd): {}", token_amount_usd);

    let lp_amount = if pool.aum_usd.to_u128() == 0 {
        token_amount_usd
    } else {
        math::checked_as_u64(
            (token_amount_usd as u128 * ctx.accounts.lp_token_mint.supply as u128)
                / pool.aum_usd.to_u128(),
        )?
    };
    msg!("Lp amount: {}", lp_amount);

    Ok(AmountAndFee {
        amount: lp_amount,
        fee: fee_amount,
    })
}
