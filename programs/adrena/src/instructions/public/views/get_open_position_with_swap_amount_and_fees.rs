use {
    super::{get_swap_amount_and_fees::GetSwapAmountAndFeesParams, GetEntryPriceAndFeeParams},
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::{Cortex, OpenPositionWithSwapAmountAndFees},
            custody::Custody,
            oracle::Oracle,
            pool::Pool,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetOpenPositionWithSwapAmountAndFees<'info> {
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
        mut,
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

    /// #6
    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 principal_custody.load()?.mint.as_ref()],
        bump = principal_custody.load()?.bump
    )]
    pub principal_custody: AccountLoader<'info, Custody>,

    /// #7
    pub adrena_program: Program<'info, Adrena>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetOpenPositionWithSwapAmountAndFeesParams {
    pub collateral_amount: u64,
    pub leverage: u32, // in BPS
    pub side: u8,      // Side
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_open_position_with_swap_amount_and_fees(
    ctx: Context<GetOpenPositionWithSwapAmountAndFees>,
    params: &GetOpenPositionWithSwapAmountAndFeesParams,
) -> Result<OpenPositionWithSwapAmountAndFees> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let cortex = ctx.accounts.cortex.load()?;
    let current_time = cortex.get_time()?;

    msg!(
        "Collateral: {} / Leverage: {}",
        params.collateral_amount,
        params.leverage
    );

    let swap_required = ctx
        .accounts
        .receiving_custody
        .key()
        .ne(&ctx.accounts.collateral_custody.key());

    // Preliminary checks
    {
        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }

        drop(oracle);
    }

    // Calculate swap fee
    let (swap_collateral_amount, swap_fee_in, swap_fee_out) = if swap_required {
        let swap_amount_and_fee = cortex.internal_get_swap_amount_and_fee(
            ctx.accounts.cortex.to_account_info(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.receiving_custody.to_account_info(),
            ctx.accounts.oracle.to_account_info(),
            ctx.accounts.collateral_custody.to_account_info(),
            ctx.accounts.adrena_program.to_account_info(),
            GetSwapAmountAndFeesParams {
                amount_in: params.collateral_amount,
                oracle_prices: None,
            },
        )?;

        (
            swap_amount_and_fee.amount_out,
            swap_amount_and_fee.fee_in,
            swap_amount_and_fee.fee_out,
        )
    } else {
        (params.collateral_amount, 0u64, 0u64)
    };

    msg!(
        "Swap collateral amount: {}, Swap fee in: {}, Swap fee out: {}",
        swap_collateral_amount,
        swap_fee_in,
        swap_fee_out
    );

    let entry_price_and_fee = cortex.internal_get_entry_price_and_fee(
        ctx.accounts.cortex.to_account_info(),
        ctx.accounts.pool.to_account_info(),
        ctx.accounts.principal_custody.to_account_info(),
        ctx.accounts.oracle.to_account_info(),
        ctx.accounts.collateral_custody.to_account_info(),
        ctx.accounts.adrena_program.to_account_info(),
        GetEntryPriceAndFeeParams {
            collateral: swap_collateral_amount,
            leverage: params.leverage,
            side: params.side,
            oracle_prices: None,
        },
    )?;

    // Information are not entirely correct, as the swap impacts are not taken into account in the
    // calculations of entry_price_and_fee (custody utilization, ratios target etc.)
    // still it's a very close estimation
    Ok(OpenPositionWithSwapAmountAndFees {
        entry_price: entry_price_and_fee.entry_price,
        liquidation_price: entry_price_and_fee.liquidation_price,
        swap_fee_in,
        swap_fee_out,
        exit_fee: entry_price_and_fee.exit_fee,
        liquidation_fee: entry_price_and_fee.liquidation_fee,
        size: entry_price_and_fee.size,
    })
}
