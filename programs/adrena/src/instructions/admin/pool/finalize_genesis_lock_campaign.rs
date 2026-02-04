use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex,
            genesis_lock::GenesisLock,
            pool::{Pool, PoolLiquidityState},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
};

#[derive(Accounts)]
pub struct FinalizeGenesisLockCampaign<'info> {
    /// #1
    /// CHECK: Anyone
    #[account(mut)]
    pub caller: Signer<'info>,

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
        seeds = [b"genesis_lock", pool.key().as_ref()],
        bump = genesis_lock.load()?.bump
    )]
    pub genesis_lock: AccountLoader<'info, GenesisLock>,

    /// #5
    system_program: Program<'info, System>,

    /// #6
    token_program: Program<'info, Token>,

    /// #7
    adrena_program: Program<'info, Adrena>,
}

pub fn finalize_genesis_lock_campaign<'info>(
    ctx: Context<'_, '_, '_, 'info, FinalizeGenesisLockCampaign<'info>>,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    let genesis_lock = ctx.accounts.genesis_lock.load()?;

    // Validation
    {
        // The Pool is initialized
        require!(pool.is_initialized(), AdrenaError::InvalidPoolState);

        // The campaign is on-going
        require!(
            pool.get_liquidity_state() == PoolLiquidityState::GenesisLiquidity,
            AdrenaError::InvalidPoolState
        );

        // Check that the campaign has ended
        require!(
            genesis_lock.has_campaign_ended()?,
            AdrenaError::InvalidGenesisLockState
        );
    }

    // When the genesis lock campaign is finalized, the pool is set to idle
    // It needs a manual action to change the state to active
    pool.liquidity_state = PoolLiquidityState::Idle.into();

    Ok(())
}
