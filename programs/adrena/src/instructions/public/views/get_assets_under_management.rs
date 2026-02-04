use {
    crate::{
        error::AdrenaError,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, oracle::Oracle, pool::Pool,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetAssetsUnderManagement<'info> {
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
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetAssetsUnderManagementParams {
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

// This is a semi-view function. Having oracle_prices mutable for code simplicity
pub fn get_assets_under_management(
    ctx: Context<GetAssetsUnderManagement>,
    params: &GetAssetsUnderManagementParams,
) -> Result<u128> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // Pick freshest price after we checked the given prices are correct
    if let Some(ref prices) = params.oracle_prices {
        oracle.verify_and_update_prices(prices, current_time)?;
    }

    ctx.accounts.pool.load()?.get_assets_under_management_usd(
        &oracle,
        ctx.remaining_accounts,
        current_time,
    )
}
