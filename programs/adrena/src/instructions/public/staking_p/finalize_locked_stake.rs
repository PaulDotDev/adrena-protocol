use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        events::FinalizeLockedStakeEvent,
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            pool::FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP,
            staking::{Staking, StakingType},
            user_staking::UserStaking,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
pub struct FinalizeLockedStake<'info> {
    /// #1
    /// CHECK: Checked within the instruction
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #2
    /// CHECK: verified through the `user_staking` account seed derivation
    #[account(mut)]
    pub owner: AccountInfo<'info>,

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
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #5
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #6
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #8
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #9
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #10
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #11
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #12
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #13
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #14
    pub adrena_program: Program<'info, Adrena>,

    /// #15
    pub system_program: Program<'info, System>,

    /// #16
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct FinalizeLockedStakeParams {
    pub locked_stake_id: u64,
    pub early_exit: bool,
}

// Finalize a stake means cancelling the governing power related to the stake and stopping to accrue rewards
// A stake can be finalized when its locking period have ended
// After a stake is finalized, it can be removed by the user to retrieve its tokens
pub fn finalize_locked_stake<'info>(
    ctx: Context<'_, '_, '_, 'info, FinalizeLockedStake<'info>>,
    params: &FinalizeLockedStakeParams,
) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;
    let mut user_staking = ctx.accounts.user_staking.load_mut()?;
    let cortex = ctx.accounts.cortex.load_mut()?;
    let current_time = cortex.get_time()?;

    // Set the staking type if not already set - this can happens for stakes created before the staking type was introduced
    if user_staking.staking_type == 0 {
        user_staking.staking_type = staking.staking_type;
    }

    let locked_stake = user_staking
        .locked_stakes
        .iter_mut()
        .find(|stake| stake.id == params.locked_stake_id)
        .ok_or(AdrenaError::UserStakeNotFound)?;

    // Preliminary checks
    {
        require!(
            locked_stake.is_initialized(),
            AdrenaError::InvalidStakeState
        );

        // In the context of permissionless execution
        if params.early_exit {
            // Cannot exit early an ended stake
            if locked_stake.has_ended(current_time)? {
                return Err(AdrenaError::InvalidStakeState.into());
            }

            // If early, owner needs to be the one to call the ix
            if !ctx.accounts.caller.key.eq(ctx.accounts.owner.key) {
                return Err(ProgramError::InvalidArgument.into());
            }

            // The locked stake must be "established" (see LockedStake.qualified_for_rewards_in_resolved_rounds_count for info)
            require!(
                locked_stake.is_established(),
                AdrenaError::StakeNotEstablished
            );
        }

        require!(
            params.early_exit || locked_stake.has_ended(current_time)?,
            AdrenaError::InvalidStakeState
        );

        require!(!locked_stake.is_resolved(), AdrenaError::InvalidStakeState);
    }

    // Revoke governing power allocated to the stake
    {
        let voting_power = math::checked_as_u64(
            (locked_stake.amount as u128 * locked_stake.vote_multiplier as u128)
                / Cortex::BPS_POWER,
        )?;

        if voting_power > 0 {
            cortex.remove_governing_power(
                ctx.accounts.transfer_authority.to_account_info(),
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
            )?;
        }
    }

    // Forfeit current round participation if staked for the current round
    if locked_stake.qualifies_for_rewards_from(&staking.current_staking_round) {
        staking.current_staking_round.total_stake -= locked_stake.amount_with_reward_multiplier;

        staking.current_staking_round.lm_total_stake -=
            locked_stake.amount_with_lm_reward_multiplier;
    }

    // Remove staked tokens from next round
    {
        staking.next_staking_round.total_stake -= locked_stake.amount_with_reward_multiplier;

        staking.next_staking_round.lm_total_stake -= locked_stake.amount_with_lm_reward_multiplier;
    }

    staking.nb_locked_tokens -= locked_stake.amount;

    locked_stake.resolved = true as u8;

    locked_stake.early_exit_fee = if params.early_exit {
        if staking.get_staking_type() == StakingType::LP
            && current_time >= FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP
        {
            // No early exit tax for LP staking after genesis is over
            // We now have liquid ALP
            0
        } else {
            locked_stake.calculate_early_exit_fee_amount(current_time)?
        }
    } else {
        0
    };

    locked_stake.early_exit = params.early_exit as u8;

    // End reward accrual if stake is finalized early
    if locked_stake.is_early_exit() {
        locked_stake.end_time = current_time;
    }

    msg!(
        "Next staking round total stake: {}",
        staking.next_staking_round.total_stake
    );
    msg!(
        "Next staking round: lm total stake: {}",
        staking.next_staking_round.lm_total_stake
    );
    msg!(
        "Resolved staking round after remove stake {:?}",
        staking.resolved_staking_rounds
    );
    msg!(
        "Current staking round after remove stake {:?}",
        staking.current_staking_round
    );
    msg!(
        "Next staking round after remove stake {:?}",
        staking.next_staking_round
    );

    emit!(FinalizeLockedStakeEvent {
        owner: ctx.accounts.owner.key(),
        staking: ctx.accounts.staking.key(),
        locked_stake_id: locked_stake.id,
        early_exit: params.early_exit,
    });

    Ok(())
}
