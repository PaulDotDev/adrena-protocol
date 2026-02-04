use {
    crate::{
        error::AdrenaError,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::{Cortex, SwapAmountAndFees},
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
        },
    },
    anchor_lang::prelude::*,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct GetSwapAmountAndFees<'info> {
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
                 receiving_custody.load()?.mint.as_ref()],
        bump = receiving_custody.load()?.bump
    )]
    pub receiving_custody: AccountLoader<'info, Custody>,

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
                 dispensing_custody.load()?.mint.as_ref()],
        bump = dispensing_custody.load()?.bump
    )]
    pub dispensing_custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetSwapAmountAndFeesParams {
    pub amount_in: u64,
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_swap_amount_and_fees(
    ctx: Context<GetSwapAmountAndFees>,
    params: &GetSwapAmountAndFeesParams,
) -> Result<SwapAmountAndFees> {
    let receiving_custody_key = ctx.accounts.receiving_custody.key();
    let dispensing_custody_key = ctx.accounts.dispensing_custody.key();
    let oracle = ctx.accounts.oracle.load()?;

    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Preliminary checks
    {
        if params.amount_in == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    require_keys_neq!(receiving_custody_key, dispensing_custody_key);

    // The mut is only local, this is a read_only view
    let receiving_custody = *ctx.accounts.receiving_custody.load()?;
    // The mut is only local, this is a read_only view
    let dispensing_custody = *ctx.accounts.dispensing_custody.load()?;

    let pool = ctx.accounts.pool.load()?;

    // Load the oracle prices
    let (received_token_price, dispensed_token_price) = {
        match &params.oracle_prices {
            Some(p) => {
                p.verify_signature()?;

                let ret = oracle.get_up_to_date_prices_readonly(
                    p,
                    vec![receiving_custody.oracle, dispensing_custody.oracle],
                    current_time,
                )?;

                (ret[0], ret[1])
            }
            None => (
                oracle.get_oracle_price(receiving_custody.oracle, current_time)?,
                oracle.get_oracle_price(dispensing_custody.oracle, current_time)?,
            ),
        }
    };

    let amount_out = pool.get_swap_amount(
        &received_token_price.low(),
        &dispensed_token_price.high(),
        &receiving_custody,
        &dispensing_custody,
        params.amount_in,
    )?;

    // calculate fee in
    let fees_in_amount =
        pool.get_swap_in_fees(params.amount_in, &receiving_custody, &dispensing_custody)?;

    // calculate fee out
    let fees_out_amount =
        pool.get_swap_out_fees(amount_out, &receiving_custody, &dispensing_custody)?;

    Ok(SwapAmountAndFees {
        amount_out,
        fee_in: fees_in_amount,
        fee_out: fees_out_amount,
    })
}
