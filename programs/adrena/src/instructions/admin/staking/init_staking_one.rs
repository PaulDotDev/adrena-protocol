use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingInitializationStep, StakingRound, StakingType},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitStakingOne<'info> {
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
        init,
        payer = payer,
        space = Staking::LEN,
        seeds = [b"staking", staking_staked_token_mint.key().as_ref()],
        bump
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
        token::mint = staking_staked_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump
    )]
    pub staking_staked_token_vault: Box<Account<'info, TokenAccount>>,

    /// #8
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #9
    #[account(
        mint::authority = transfer_authority,
    )]
    pub staking_staked_token_mint: Box<Account<'info, Mint>>,

    /// #10
    pub adrena_program: Program<'info, Adrena>,

    /// #11
    system_program: Program<'info, System>,

    /// #12
    token_program: Program<'info, Token>,

    /// #13
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitStakingOneParams {
    pub staking_type: u8, // StakingType
}

pub fn init_staking_one<'info>(
    ctx: Context<'_, '_, '_, 'info, InitStakingOne<'info>>,
    params: &InitStakingOneParams,
) -> Result<u8> {
    let mut staking = ctx.accounts.staking.load_init()?;

    staking.bump = ctx.bumps.staking;
    staking.staked_token_vault_bump = ctx.bumps.staking_staked_token_vault;

    staking.initialized = StakingInitializationStep::Step1.into();

    let staking_type = StakingType::try_from(params.staking_type)?;

    staking.staking_type = staking_type.into();
    staking.staked_token_mint = ctx.accounts.staking_staked_token_mint.key();
    staking.staked_token_decimals = ctx.accounts.staking_staked_token_mint.decimals;

    staking.reward_token_decimals = ctx.accounts.fee_redistribution_mint.decimals;

    staking.resolved_reward_token_amount = u64::MIN;
    staking.resolved_staked_token_amount = u64::MIN;
    staking.resolved_lm_reward_token_amount = u64::MIN;
    staking.resolved_lm_staked_token_amount = u64::MIN;

    staking.current_staking_round = StakingRound::new(ctx.accounts.cortex.load()?.get_time()?);

    staking.next_staking_round = Default::default();
    staking.resolved_staking_rounds = Default::default();

    staking.lm_emission_potentiometer_bps = Staking::LM_EMISSION_POTENTIOMETER_BASELINE_BPS; // 100%
    staking.months_elapsed_since_inception = 0;
    staking.emission_amount_per_round_last_update = Clock::get()?.unix_timestamp;

    match staking_type {
        StakingType::LM => {
            staking.current_month_emission_amount_per_round =
                Staking::LM_STAKING_REWARDS_EMISSION_FIRST_MONTH_AMOUNT
                    / crate::state::staking::MAX_ROUNDS_PER_MONTH;
        }
        StakingType::LP => {
            staking.current_month_emission_amount_per_round =
                Staking::LP_STAKING_REWARDS_EMISSION_FIRST_MONTH_AMOUNT
                    / crate::state::staking::MAX_ROUNDS_PER_MONTH;
        }
    }

    Ok(0)
}
