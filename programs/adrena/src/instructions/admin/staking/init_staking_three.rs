use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingInitializationStep},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitStakingThree<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: UncheckedAccount<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.get_initialized() == StakingInitializationStep::Step2 @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #5
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #6
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        has_one = fee_redistribution_mint,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        init,
        payer = payer,
        token::mint = lm_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #8
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #9
    system_program: Program<'info, System>,

    /// #10
    token_program: Program<'info, Token>,
}

pub fn init_staking_three<'info>(
    ctx: Context<'_, '_, '_, 'info, InitStakingThree<'info>>,
) -> Result<u8> {
    let mut staking = ctx.accounts.staking.load_mut()?;

    staking.lm_reward_token_vault_bump = ctx.bumps.staking_lm_reward_token_vault;

    staking.initialized = StakingInitializationStep::Step3.into();

    Ok(0)
}
