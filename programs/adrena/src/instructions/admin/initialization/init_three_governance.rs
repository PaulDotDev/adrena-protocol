use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        state::cortex::{Cortex, CortexInitializationStep},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
pub struct InitThreeGovernance<'info> {
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
        constraint = cortex.load()?.get_initialized() == CortexInitializationStep::Step2 @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    // the shadow governance token transparently managed by the program (and only the program)
    #[account(
        init,
        payer = payer,
        mint::authority = transfer_authority,
        mint::freeze_authority = transfer_authority,
        mint::decimals = Cortex::GOVERNANCE_SHADOW_TOKEN_DECIMALS,
        seeds = [b"governance_token_mint"],
        bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #6
    /// CHECK: Checked by spl governance v3 program later on
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #7
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,

    /// #10
    rent: Sysvar<'info, Rent>,
}

pub fn init_three_governance<'info>(
    ctx: Context<'_, '_, '_, 'info, InitThreeGovernance<'info>>,
) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.inception_time = cortex.get_time()?;
    cortex.initialized = CortexInitializationStep::Step3.into();

    cortex.governance_program = ctx.accounts.governance_program.key();
    cortex.governance_realm = ctx.accounts.governance_realm.key();
    cortex.governance_token_bump = ctx.bumps.governance_token_mint;

    Ok(())
}
