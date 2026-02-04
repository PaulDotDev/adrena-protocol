use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingInitializationStep},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitStakingFour<'info> {
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
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"staking", staking_staked_token_mint.key().as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.get_initialized() == StakingInitializationStep::Step3 @AdrenaError::InvalidStakingState
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
        mut,
        token::mint = staking_staked_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump = staking.load()?.staked_token_vault_bump
    )]
    pub staking_staked_token_vault: Box<Account<'info, TokenAccount>>,

    /// #8
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #9
    #[account(
        mut,
        token::mint = lm_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #10
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #11
    #[account(
        mint::authority = transfer_authority,
    )]
    pub staking_staked_token_mint: Box<Account<'info, Mint>>,

    /// #12
    pub adrena_program: Program<'info, Adrena>,

    /// #13
    system_program: Program<'info, System>,

    /// #14
    token_program: Program<'info, Token>,
}

pub fn init_staking_four<'info>(
    ctx: Context<'_, '_, '_, 'info, InitStakingFour<'info>>,
) -> Result<u8> {
    let mut staking = ctx.accounts.staking.load_mut()?;

    staking.initialized = StakingInitializationStep::Initialized.into();

    Ok(0)
}
