use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        events::AddLockedStakeEvent,
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingType},
            user_staking::{LockedStake, UserStaking},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct AddLockedStake<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        token::mint = staking.load()?.staked_token_mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #3
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        has_one = owner
    )]
    pub reward_token_account: Box<Account<'info, TokenAccount>>,

    /// #4
    #[account(
        mut,
        token::mint = staking.load()?.staked_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump = staking.load()?.staked_token_vault_bump,
    )]
    pub staking_staked_token_vault: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #6
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #7
    #[account(
        mut,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #8
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #9
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = fee_redistribution_mint,
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #10
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #11
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #12
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #13
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #14
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #15
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #16
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #17
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #18
    pub adrena_program: Program<'info, Adrena>,

    /// #19
    pub system_program: Program<'info, System>,

    /// #20
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct AddLockedStakeParams {
    pub amount: u64,
    // Amount of days to be locked for
    pub locked_days: u32,
}

pub fn add_locked_stake(ctx: Context<AddLockedStake>, params: &AddLockedStakeParams) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;

    let staking_option = {
        if params.amount == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        UserStaking::get_locked_staking_option(params.locked_days, staking.get_staking_type())
    }?;

    let mut user_staking = ctx.accounts.user_staking.load_mut()?;
    let cortex = ctx.accounts.cortex.load_mut()?;

    // Validate inputs
    {
        // Only authorize LM staking - Now ALP is fully liquid
        if staking.get_staking_type() != StakingType::LM {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    // Set the staking type if not already set - this can happens for stakes created before the staking type was introduced
    if user_staking.staking_type == 0 {
        user_staking.staking_type = staking.staking_type;
    }

    // Verify that the locked stake for that ID does not already exist
    let locked_stake_id = user_staking.get_next_locked_stake_id();

    // Add stake to UserStaking account
    let (stake_amount_with_reward_multiplier, stake_amount_with_lm_reward_multiplier) = {
        let stake_amount_with_reward_multiplier = math::checked_as_u64(
            (params.amount as u128 * staking_option.reward_multiplier as u128) / Cortex::BPS_POWER,
        )?;

        let stake_amount_with_lm_reward_multiplier = math::checked_as_u64(
            (params.amount as u128 * staking_option.lm_reward_multiplier as u128)
                / Cortex::BPS_POWER,
        )?;

        let current_time = cortex.get_time()?;
        // Setup the new LockedStake
        let new_locked_stake = LockedStake {
            amount: params.amount,
            stake_time: current_time,
            claim_time: 0,
            end_time: current_time + staking_option.locked_days as i64 * 3_600 * 24,

            // Transform days in seconds here
            lock_duration: staking_option.locked_days as u64 * 3_600 * 24,
            reward_multiplier: staking_option.reward_multiplier,
            lm_reward_multiplier: staking_option.lm_reward_multiplier,
            vote_multiplier: staking_option.vote_multiplier,

            amount_with_reward_multiplier: stake_amount_with_reward_multiplier,
            amount_with_lm_reward_multiplier: stake_amount_with_lm_reward_multiplier,

            resolved: false as u8,
            id: locked_stake_id,

            early_exit: false as u8,
            early_exit_fee: 0,

            is_genesis: false as u8,
            genesis_claim_time: 0,
            ..LockedStake::default()
        };

        // Add the new locked staking to the list
        user_staking.add_locked_stake(new_locked_stake)?;

        (
            stake_amount_with_reward_multiplier,
            stake_amount_with_lm_reward_multiplier,
        )
    };

    // transfer newly staked tokens to Stake PDA
    cortex.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.staking_staked_token_vault.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    //           LM Staking
    //   ---------------------------
    //   voting power         | *x |
    //   real yield rewards   | *y |
    //   lm rewards           | *z |
    //   ---------------------------
    //
    //           LP Staking
    //   ---------------------------
    //   voting power         |  0 |
    //   real yield rewards   | *i |
    //   lm rewards           | *j |
    //   ---------------------------
    {
        // If LM staking, give governing power to the Stake owner
        if staking.get_staking_type() == StakingType::LM {
            // Apply voting multiplier related to locking period
            let voting_power = math::checked_as_u64(
                (params.amount as u128 * staking_option.vote_multiplier as u128)
                    / Cortex::BPS_POWER,
            )?;

            cortex.add_governing_power(
                ctx.accounts.transfer_authority.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts
                    .governance_governing_token_owner_record
                    .to_account_info(),
                ctx.accounts.governance_token_mint.to_account_info(),
                ctx.accounts.governance_realm.to_account_info(),
                ctx.accounts.governance_realm_config.to_account_info(),
                ctx.accounts
                    .governance_governing_token_holding
                    .to_account_info(),
                ctx.accounts.governance_program.to_account_info(),
                voting_power,
                None,
                true,
            )?;
        }

        staking.next_staking_round.total_stake += stake_amount_with_reward_multiplier;

        staking.next_staking_round.lm_total_stake += stake_amount_with_lm_reward_multiplier;

        staking.nb_locked_tokens += params.amount;
    }

    emit!(AddLockedStakeEvent {
        owner: ctx.accounts.owner.key(),
        staking: ctx.accounts.staking.key(),
        locked_stake_id,
        amount: params.amount,
        locked_days: params.locked_days,
    });

    Ok(())
}
