use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, custody::Custody, pool::Pool},
    },
    anchor_lang::prelude::*,
    std::str::FromStr,
};

/// The goal of this instruction is to doctor data in the custody accounts to match the locked
/// This should be ran once, have no effect if ran again as the two ix should always be in line
/// To be removed during next release
#[derive(Accounts)]
pub struct PatchCustodyLockedAmount<'info> {
    /// #1
    pub admin: Signer<'info>,

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
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap().as_ref()],
        bump
    )]
    pub usdc_custody: AccountLoader<'info, Custody>,

    /// #5
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap().as_ref()],
        bump
    )]
    pub bonk_custody: AccountLoader<'info, Custody>,

    /// #6
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 Pubkey::from_str("3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh").unwrap().as_ref()],
        bump
    )]
    pub wbtc_custody: AccountLoader<'info, Custody>,

    /// #7
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 Pubkey::from_str("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn").unwrap().as_ref()],
        bump
    )]
    pub jito_custody: AccountLoader<'info, Custody>,
}

pub fn patch_custody_locked_amount(_ctx: Context<PatchCustodyLockedAmount>) -> Result<()> {
    // Patched - kept for IDL and history
    // {
    //     let mut custody = ctx.accounts.usdc_custody.load_mut()?;
    //     custody.assets.locked = custody.short_positions.locked_amount;
    // }

    // {
    //     let mut custody = ctx.accounts.bonk_custody.load_mut()?;
    //     custody.assets.locked = custody.long_positions.locked_amount;
    // }

    // {
    //     let mut custody = ctx.accounts.wbtc_custody.load_mut()?;
    //     custody.assets.locked = custody.long_positions.locked_amount;
    // }

    // {
    //     let mut custody = ctx.accounts.jito_custody.load_mut()?;
    //     custody.assets.locked = custody.long_positions.locked_amount;
    // }

    Ok(())
}
