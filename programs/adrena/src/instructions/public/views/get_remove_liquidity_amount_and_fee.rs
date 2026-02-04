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
pub struct GetRemoveLiquidityAmountAndFee<'info> {
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

    /// #5
    #[account(
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #6
    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetRemoveLiquidityAmountAndFeeParams {
    pub lp_amount_in: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_remove_liquidity_amount_and_fee(
    ctx: Context<GetRemoveLiquidityAmountAndFee>,
    params: &GetRemoveLiquidityAmountAndFeeParams,
) -> Result<AmountAndFee> {
    let oracle = ctx.accounts.oracle.load()?;

    // Preliminary checks
    {
        if params.lp_amount_in == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }
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

    let remove_amount_usd = math::checked_as_u64(
        (pool.aum_usd.to_u128() * params.lp_amount_in as u128)
            / ctx.accounts.lp_token_mint.supply as u128,
    )?;

    let remove_amount = token_price
        .low()
        .get_token_amount(remove_amount_usd, custody.decimals)?;

    let fee_amount = pool.get_remove_liquidity_fee(remove_amount, &custody)?;

    let transfer_amount = remove_amount - fee_amount;

    Ok(AmountAndFee {
        amount: transfer_amount,
        fee: fee_amount,
    })
}
