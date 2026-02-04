use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        events::UpgradeLockedStakeEvent,
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            genesis_lock::GenesisLock,
            pool::Pool,
            staking::{Staking, StakingType},
            user_staking::UserStaking,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

/// This instruction let a use increase their locked stake in amount of tokens locked and in quality (the lock period)
/// Any change will result in the reset of the time spent locked in
#[derive(Accounts)]
pub struct UpgradeLockedStake<'info> {
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
        token::mint = lm_token_mint,
        has_one = owner
    )]
    pub lm_token_account: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        token::mint = staking.load()?.staked_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump = staking.load()?.staked_token_vault_bump,
    )]
    pub staking_staked_token_vault: Box<Account<'info, TokenAccount>>,

    /// #6
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #7
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #8
    #[account(
        mut,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #9
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #10
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = fee_redistribution_mint,
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #11
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

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
        mut,
        seeds = [b"genesis_lock", pool.key().as_ref()],
        bump = genesis_lock.load()?.bump
    )]
    pub genesis_lock: AccountLoader<'info, GenesisLock>,

    /// #14
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #15
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #16
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #17
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #18
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #19
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #20
    #[account(
        mut,
        token::mint = lm_token_mint,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.lm_reward_token_vault_bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #21
    pub adrena_program: Program<'info, Adrena>,

    /// #22
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #23
    pub system_program: Program<'info, System>,

    /// #24
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct UpgradeLockedStakeParams {
    pub locked_stake_id: u64,
    // Additional amount to add to the locked stake
    pub amount: Option<u64>,
    // New lock duration in days (can only be > to current lock duration)
    pub locked_days: Option<u32>,
}

pub fn upgrade_locked_stake(
    ctx: Context<UpgradeLockedStake>,
    params: &UpgradeLockedStakeParams,
) -> Result<()> {
    // Claim existing rewards before upgrading
    {
        let cpi_accounts = crate::cpi::accounts::ClaimStakes {
            caller: ctx.accounts.owner.to_account_info(),
            payer: ctx.accounts.owner.to_account_info(),
            owner: ctx.accounts.owner.to_account_info(),
            reward_token_account: ctx.accounts.reward_token_account.to_account_info(),
            lm_token_account: ctx.accounts.lm_token_account.to_account_info(),
            staking_reward_token_vault: ctx.accounts.staking_reward_token_vault.to_account_info(),
            staking_lm_reward_token_vault: ctx
                .accounts
                .staking_lm_reward_token_vault
                .to_account_info(),
            transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
            user_staking: ctx.accounts.user_staking.to_account_info(),
            staking: ctx.accounts.staking.to_account_info(),
            cortex: ctx.accounts.cortex.to_account_info(),
            pool: ctx.accounts.pool.to_account_info(),
            genesis_lock: ctx.accounts.genesis_lock.to_account_info(),
            lm_token_mint: ctx.accounts.lm_token_mint.to_account_info(),
            fee_redistribution_mint: ctx.accounts.fee_redistribution_mint.to_account_info(),
            adrena_program: ctx.accounts.adrena_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        let cpi_program = ctx.accounts.adrena_program.to_account_info();
        crate::cpi::claim_stakes(
            CpiContext::new(cpi_program, cpi_accounts),
            crate::ClaimStakesParams {
                locked_stake_indexes: None,
            },
        )?;

        // Force reloading all accounts that may have been affected by claim
        {
            ctx.accounts.reward_token_account.reload()?;
            ctx.accounts.lm_token_account.reload()?;
            ctx.accounts.staking_reward_token_vault.reload()?;
            ctx.accounts.staking_lm_reward_token_vault.reload()?;
            ctx.accounts.lm_token_mint.reload()?;
            ctx.accounts.fee_redistribution_mint.reload()?;
        }
    }

    let mut staking = ctx.accounts.staking.load_mut()?;
    let mut user_staking = ctx.accounts.user_staking.load_mut()?;
    let cortex = ctx.accounts.cortex.load_mut()?;
    let current_time = cortex.get_time()?;

    // Find the targeted Locked Stake instance (matched by id)
    let locked_stake = user_staking
        .locked_stakes
        .iter_mut()
        .find(|stake| stake.id == params.locked_stake_id)
        .ok_or(AdrenaError::UserStakeNotFound)?;

    // Save the voting power for updating only the added delta later
    let voting_power_before_change = math::checked_as_u64(
        (locked_stake.amount as u128 * locked_stake.vote_multiplier as u128) / Cortex::BPS_POWER,
    )?;

    // Preliminary checks
    {
        // Must provide either amount or locked_days. Else it's a no-op
        if params.amount.is_none() && params.locked_days.is_none() {
            return Err(ProgramError::InvalidArgument.into());
        };

        // Checking stake state
        // Must be initialized (an already existing locked stake)
        // Must not be a genesis instance
        // Must not be in early-exit state
        // Must not be a resolved Locked Stake (terminated)
        // Must not be ended (beyond its due date)
        require!(
            locked_stake.is_initialized()
                && !locked_stake.is_genesis()
                && !locked_stake.is_early_exit()
                && !locked_stake.is_resolved()
                && !locked_stake.has_ended(current_time)?,
            AdrenaError::InvalidStakeState
        );

        // If there is a value in params.amount, verify it's non zero
        if let Some(amount) = params.amount {
            if amount == 0 {
                return Err(ProgramError::InvalidArgument.into());
            }
        }

        // If there is a value in params.locked_days, verify it's non zero - Checked below in 2-1

        // Requires the stake to be established (see LockedStake.qualified_for_rewards_in_resolved_rounds_count for info)
        require!(
            locked_stake.is_established(),
            AdrenaError::StakeNotEstablished
        );
    }

    // Defines the new amount and lock duration based on the provided parameters
    let (new_amount, new_locked_days, new_staking_option) = {
        let new_total_amount = params
            .amount
            .map(|additional_amount| locked_stake.amount + additional_amount)
            .unwrap_or(locked_stake.amount);
        msg!(
            "new total amount: {} (previous {})",
            new_total_amount,
            locked_stake.amount
        );

        // Now we can determine the new lock duration in seconds (if provided, else stay the same)
        let current_lock_duration = locked_stake.lock_duration;
        let current_locked_days = (current_lock_duration / 3600 / 24) as u32;
        let new_locked_days = match params.locked_days {
            Some(new_locked_days) => {
                let new_lock_duration = new_locked_days as u64 * 3_600 * 24;
                // If provided, must be superior to the current lock duration
                require!(
                    new_lock_duration > current_lock_duration,
                    AdrenaError::InvalidLockDuration
                );
                msg!(
                    "new locked days: {} (previous {})",
                    new_locked_days,
                    current_locked_days
                );
                new_locked_days
            }
            // Keep the current lock_duration
            None => {
                msg!("current locked days: {}", current_locked_days);
                current_locked_days
            }
        };

        // Generate the associated LockedStakingOption - will implicitly check that the parameter provided match one of the possible durations
        let new_staking_option =
            UserStaking::get_locked_staking_option(new_locked_days, staking.get_staking_type())?;

        (new_total_amount, new_locked_days, new_staking_option)
    };

    // Calculate rewards w/ multipliers
    let (
        new_stake_amount_with_reward_multiplier,
        new_stake_amount_with_lm_reward_multiplier,
        additional_stake_amount_with_reward_multiplier,
        additional_stake_amount_with_lm_reward_multiplier,
    ) = {
        let old_stake_amount_with_reward_multiplier = locked_stake.amount_with_reward_multiplier;
        let old_stake_amount_with_lm_reward_multiplier =
            locked_stake.amount_with_lm_reward_multiplier;

        let new_stake_amount_with_reward_multiplier = math::checked_as_u64(
            (new_amount as u128 * new_staking_option.reward_multiplier as u128) / Cortex::BPS_POWER,
        )?;
        let new_stake_amount_with_lm_reward_multiplier = math::checked_as_u64(
            (new_amount as u128 * new_staking_option.lm_reward_multiplier as u128)
                / Cortex::BPS_POWER,
        )?;

        (
            new_stake_amount_with_reward_multiplier,
            new_stake_amount_with_lm_reward_multiplier,
            new_stake_amount_with_reward_multiplier - old_stake_amount_with_reward_multiplier,
            new_stake_amount_with_lm_reward_multiplier - old_stake_amount_with_lm_reward_multiplier,
        )
    };

    // Update the locked stake
    // When doing so, treat the whole stake as a new stake, given either the amount or the lock duration changed (or both).
    // In both case, we will be updating the stake amount with the new reward multipliers, and the user will receive boosted reward on the current round.
    // Add stake to UserStaking account
    {
        let new_lock_duration = new_locked_days as u64 * 3_600 * 24;

        // Update the locked stake amount and duration
        locked_stake.amount = new_amount;
        locked_stake.lock_duration = new_lock_duration;

        //  Update the end time to reflect the new lock duration
        // In any case, the lock duration is reset
        locked_stake.end_time = current_time + new_lock_duration as i64;

        // Grant increased rewards and voting multiplier to match the updated amount and LockedStakingOption
        locked_stake.reward_multiplier = new_staking_option.reward_multiplier;
        locked_stake.lm_reward_multiplier = new_staking_option.lm_reward_multiplier;
        locked_stake.vote_multiplier = new_staking_option.vote_multiplier;
        locked_stake.amount_with_reward_multiplier = new_stake_amount_with_reward_multiplier;
        locked_stake.amount_with_lm_reward_multiplier = new_stake_amount_with_lm_reward_multiplier;
    }

    // Transfer any additional staked tokens to Stake PDA
    if let Some(additional_amount) = params.amount {
        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts.staking_staked_token_vault.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            additional_amount,
        )?;
    }

    // Update the Staking PDA
    {
        // If LM staking, give additional governing power to the Stake owner
        if staking.get_staking_type() == StakingType::LM {
            // First calculate the new voting power, and subtract the previously held voting power, to only apply the delta
            let additional_voting_power = math::checked_as_u64(
                (locked_stake.amount as u128 * locked_stake.vote_multiplier as u128)
                    / Cortex::BPS_POWER,
            )? - voting_power_before_change;

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
                additional_voting_power,
                None,
                true,
            )?;
        }

        // Add delta to current and next round - when user upgrade, that increase rewards right away
        msg!(
            "old current_staking_round total_stake: {} / lm_total_stake: {}",
            staking.current_staking_round.total_stake,
            staking.current_staking_round.lm_total_stake
        );
        staking.current_staking_round.total_stake += additional_stake_amount_with_reward_multiplier;
        staking.current_staking_round.lm_total_stake +=
            additional_stake_amount_with_lm_reward_multiplier;
        msg!(
            "new current_staking_round total_stake: {} / lm_total_stake: {}",
            staking.current_staking_round.total_stake,
            staking.current_staking_round.lm_total_stake
        );

        msg!(
            "old next_staking_round.total_stake: {} / lm_total_stake: {}",
            staking.next_staking_round.total_stake,
            staking.next_staking_round.lm_total_stake
        );
        staking.next_staking_round.total_stake += additional_stake_amount_with_reward_multiplier;
        staking.next_staking_round.lm_total_stake +=
            additional_stake_amount_with_lm_reward_multiplier;
        msg!(
            "next_staking_round.total_stake: {} / lm_total_stake: {}",
            staking.next_staking_round.total_stake,
            staking.next_staking_round.lm_total_stake
        );

        // Update the total amount of locked tokens
        if let Some(additional_amount) = params.amount {
            staking.nb_locked_tokens += additional_amount;
        }
    }

    emit!(UpgradeLockedStakeEvent {
        owner: ctx.accounts.owner.key(),
        staking: ctx.accounts.staking.key(),
        locked_stake_id: locked_stake.id,
        amount: params.amount,
        locked_days: params.locked_days,
    });

    Ok(())
}
