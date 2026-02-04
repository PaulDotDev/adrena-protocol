use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex,
            pool::Pool,
            staking::Staking,
            user_staking::{LockedStake, UserStaking},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitUserStaking<'info> {
    /// #1
    pub caller: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Any account
    pub owner: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        has_one = owner
    )]
    pub reward_token_account: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        token::mint = lm_token_mint,
        has_one = owner
    )]
    pub lm_token_account: Box<Account<'info, TokenAccount>>,

    /// #6
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #7
    #[account(
        mut,
        token::mint = lm_token_mint,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.lm_reward_token_vault_bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #8
    #[account(
        init,
        payer = payer,
        space = UserStaking::LEN,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #9
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #10
    #[account(
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #11
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = fee_redistribution_mint,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #12
    #[account(
        mut,
        seeds = [b"pool",
                    pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #13
    #[account(
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #14
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #15
    pub adrena_program: Program<'info, Adrena>,

    /// #16
    pub system_program: Program<'info, System>,

    /// #17
    pub token_program: Program<'info, Token>,
}

pub fn init_user_staking(ctx: Context<InitUserStaking>) -> Result<()> {
    let mut user_staking = ctx.accounts.user_staking.load_init()?;

    {
        user_staking.bump = ctx.bumps.user_staking;
        user_staking.staking_type = ctx.accounts.staking.load()?.staking_type;

        user_staking.locked_stakes = [LockedStake::default(); UserStaking::MAX_LOCKED_STAKES];

        user_staking.liquid_stake.amount = u64::MIN;
        user_staking.liquid_stake.stake_time = 0;
        user_staking.liquid_stake.claim_time = 0;
        user_staking.liquid_stake.overlap_time = 0;
        user_staking.liquid_stake.overlap_amount = u64::MIN;
    }

    Ok(())
}
