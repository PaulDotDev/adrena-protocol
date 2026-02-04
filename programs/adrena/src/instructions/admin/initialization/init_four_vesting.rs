use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::{Cortex, CortexInitializationStep},
            vest_registry::VestRegistry,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
};

#[derive(Accounts)]
pub struct InitFourVesting<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, will be set as authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.get_initialized() == CortexInitializationStep::Step3 @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        init,
        payer = payer,
        space = VestRegistry::LEN,
        seeds = [b"vest_registry"],
        bump
    )]
    pub vest_registry: Box<Account<'info, VestRegistry>>,

    /// #6
    system_program: Program<'info, System>,

    /// #7
    token_program: Program<'info, Token>,

    /// #8
    rent: Sysvar<'info, Rent>,
}

pub fn init_four_vesting<'info>(
    ctx: Context<'_, '_, '_, 'info, InitFourVesting<'info>>,
) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.inception_time = cortex.get_time()?;
    cortex.initialized = CortexInitializationStep::Initialized.into();

    // Record vest registry
    {
        let vest_registry = ctx.accounts.vest_registry.as_mut();

        vest_registry.bump = ctx.bumps.vest_registry;
        vest_registry.vests = Default::default();
        vest_registry.vesting_token_amount = 0;
    }

    Ok(())
}
