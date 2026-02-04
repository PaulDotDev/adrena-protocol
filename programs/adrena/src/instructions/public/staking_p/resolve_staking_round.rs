use {
    crate::{
        error::AdrenaError,
        instructions::{BucketName, MintLmTokensFromBucketParams},
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            staking::{NextStakingRound, Staking, StakingRound, StakingType},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    num::Zero,
};

#[derive(Accounts)]
pub struct ResolveStakingRound<'info> {
    /// #1
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #2
    // Pay for realloc
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        token::mint = staking.load()?.staked_token_mint,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump = staking.load()?.staked_token_vault_bump
    )]
    pub staking_staked_token_vault: Box<Account<'info, TokenAccount>>,

    /// #4
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        token::mint = lm_token_mint,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.lm_reward_token_vault_bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

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
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #8
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = fee_redistribution_mint,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #9
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #10
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #11
    pub adrena_program: Program<'info, Adrena>,

    /// #12
    pub system_program: Program<'info, System>,

    /// #13
    pub token_program: Program<'info, Token>,
}

// Note: the rewards will go to the next round stakers if a round finishes without anyone staking
pub fn resolve_staking_round(ctx: Context<ResolveStakingRound>) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;
    let cortex = ctx.accounts.cortex.load()?;

    let transfer_authority_bump = cortex.transfer_authority_bump;
    let current_time = cortex.get_time()?;

    drop(cortex);

    // verify that the current round is eligible for resolution
    require!(
        staking.current_staking_round_is_resolvable(current_time)?,
        AdrenaError::InvalidStakingRoundState
    );

    let current_time = Clock::get()?.unix_timestamp;

    // Calculate and mint LM token rewards for current round (different if LP or LM staking - both have different LM reward emission rate)
    {
        // Update emission amount per round if needed (will happen once every 30 days)
        {
            // If one month has elapsed since `emission_amount_per_round_last_update`, update the emission amount per round
            match staking.get_staking_type() {
                StakingType::LM => {
                    staking.update_lm_emission_amount_per_round_for_lm_staking_if_needed(
                        current_time,
                    )?;
                }
                StakingType::LP => {
                    staking.update_lm_emission_amount_per_round_for_lp_staking_if_needed(
                        current_time,
                    )?;
                }
            }
        }

        // We don't want LM rewards to accumulate before the trading is live
        // 1726992000 = 22/09/2024 12am UTC (trading go live)
        #[cfg(not(feature = "test"))]
        let current_round_lm_reward_token_amount = if current_time < 1727006400 {
            0
        } else if staking.get_staking_type() == StakingType::LP
            && staking.current_staking_round.start_time
                >= crate::state::pool::FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP
        {
            // One time event where the emissions are stopped for ALP
            0
        } else {
            staking.current_month_emission_amount_per_round
        };

        #[cfg(feature = "test")]
        let current_round_lm_reward_token_amount = staking.current_month_emission_amount_per_round;

        // Adjust rewards based on emission potentiometers values
        let adjusted_current_round_lm_reward_token_amount =
            staking.apply_emission_filter(current_round_lm_reward_token_amount)?;

        // Mint LM tokens
        {
            if adjusted_current_round_lm_reward_token_amount > 0 {
                let cpi_accounts = crate::cpi::accounts::MintLmTokensFromBucket {
                    admin: ctx.accounts.transfer_authority.to_account_info(),
                    receiving_account: ctx.accounts.staking_lm_reward_token_vault.to_account_info(),
                    transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
                    cortex: ctx.accounts.cortex.to_account_info(),
                    lm_token_mint: ctx.accounts.lm_token_mint.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.adrena_program.to_account_info();
                crate::cpi::mint_lm_tokens_from_bucket(
                    CpiContext::new_with_signer(
                        cpi_program,
                        cpi_accounts,
                        &[&[b"transfer_authority", &[transfer_authority_bump]]],
                    ),
                    MintLmTokensFromBucketParams {
                        bucket_name: BucketName::Ecosystem.into(),
                        amount: adjusted_current_round_lm_reward_token_amount,
                        reason: String::from("UserStaking rewards"),
                    },
                )?;

                {
                    ctx.accounts.staking_lm_reward_token_vault.reload()?;
                    ctx.accounts.lm_token_mint.reload()?;
                }
            }
        }

        // Reload account to account for newly minted tokens
        ctx.accounts.staking_lm_reward_token_vault.reload()?;
    }

    // Calculate metrics
    let (
        current_round_reward_token_amount,
        current_round_stake_token_amount,
        current_round_lm_reward_token_amount,
        current_round_lm_stake_token_amount,
    ) = {
        // Consider as reward everything that is in the vault, minus what is already assigned as reward
        let current_round_reward_token_amount =
            ctx.accounts.staking_reward_token_vault.amount - staking.resolved_reward_token_amount;

        let current_round_stake_token_amount = staking.current_staking_round.total_stake;

        // Consider as reward everything that is in the vault, minus what is already assigned as reward
        let current_round_lm_reward_token_amount =
            ctx.accounts.staking_lm_reward_token_vault.amount
                - staking.resolved_lm_reward_token_amount;

        let current_round_lm_stake_token_amount = staking.current_staking_round.lm_total_stake;

        (
            current_round_reward_token_amount,
            current_round_stake_token_amount,
            current_round_lm_reward_token_amount,
            current_round_lm_stake_token_amount,
        )
    };

    // Calculate rates
    {
        // Rate
        match current_round_stake_token_amount {
            0 => staking.current_staking_round.rate = 0,
            _ => {
                staking.current_staking_round.rate = math::checked_decimal_div(
                    current_round_reward_token_amount,
                    -(staking.reward_token_decimals as i32),
                    current_round_stake_token_amount,
                    -(staking.staked_token_decimals as i32),
                    -(Cortex::RATE_DECIMALS as i32),
                )?
            }
        }

        msg!("Current round rate {}", staking.current_staking_round.rate);

        // lm rate
        match current_round_lm_stake_token_amount {
            0 => staking.current_staking_round.lm_rate = 0,
            _ => {
                staking.current_staking_round.lm_rate = math::checked_decimal_div(
                    current_round_lm_reward_token_amount,
                    -(Cortex::LM_DECIMALS as i32),
                    current_round_lm_stake_token_amount,
                    -(Cortex::LM_DECIMALS as i32),
                    -(Cortex::RATE_DECIMALS as i32),
                )?
            }
        }

        msg!(
            "Current round lm rate {}",
            staking.current_staking_round.lm_rate
        );
    }

    // If there are staked tokens and there are tokens to distribute
    if (!current_round_stake_token_amount.is_zero()
        || !current_round_lm_stake_token_amount.is_zero())
        && (staking.current_staking_round.rate != 0 || staking.current_staking_round.lm_rate != 0)
    {
        // Update cortex data
        {
            {
                staking.resolved_staked_token_amount += current_round_stake_token_amount;

                require!(
                    ctx.accounts.staking_reward_token_vault.amount
                        == (staking.resolved_reward_token_amount
                            + current_round_reward_token_amount),
                    AdrenaError::InvalidStakingRoundState
                );

                staking.resolved_reward_token_amount =
                    ctx.accounts.staking_reward_token_vault.amount;
            }

            {
                staking.resolved_lm_staked_token_amount += current_round_lm_stake_token_amount;

                require!(
                    ctx.accounts.staking_lm_reward_token_vault.amount
                        == (staking.resolved_lm_reward_token_amount
                            + current_round_lm_reward_token_amount),
                    AdrenaError::InvalidStakingRoundState
                );

                staking.resolved_lm_reward_token_amount =
                    ctx.accounts.staking_lm_reward_token_vault.amount;
            }
        }

        // Move current round to resolved rounds array
        {
            let mut current_staking_round = staking.current_staking_round;

            // Safety measure
            // If too many resolved staking rounds, drop the oldest
            // Should never happens as cron should auto-claim on behalf of users, cleaning resolved rounds on the way
            // However if it does happens, rewards will be redirected to current round (implicit)
            if staking.registered_resolved_staking_round_count
                == StakingRound::MAX_RESOLVED_ROUNDS as u8
            {
                msg!(
                    "MAX_RESOLVED_ROUNDS ({}) have been reached, drop oldest round",
                    StakingRound::MAX_RESOLVED_ROUNDS
                );

                // Should always have an oldest round if reached MAX_RESOLVED_ROUNDS
                let oldest_staking_round_index =
                    staking.find_oldest_resolved_staking_round().unwrap();

                // Remove round from accounting
                {
                    let oldest_round = staking.resolved_staking_rounds[oldest_staking_round_index];

                    if oldest_round.total_claim > oldest_round.total_stake {
                        // BUGFIX:
                        //
                        // Due to a bug in the upgrade_locked_stake process, it is possible that the total_claim is higher than the total_stake
                        // Ignore the offset
                        // Once 10d have passed, this check won't be necessary and will be removed from codebase
                        //
                    } else {
                        //
                        // Regular scenario
                        //

                        let stake_token_eligible_to_rewards =
                            oldest_round.total_stake - oldest_round.total_claim;

                        let unclaimed_rewards = math::checked_decimal_mul(
                            oldest_round.rate,
                            -(Cortex::RATE_DECIMALS as i32),
                            stake_token_eligible_to_rewards,
                            -(staking.staked_token_decimals as i32),
                            -(staking.reward_token_decimals as i32),
                        )?;

                        staking.resolved_reward_token_amount -= unclaimed_rewards;

                        staking.resolved_staked_token_amount -= stake_token_eligible_to_rewards;
                    }
                }

                msg!(
                    "Removed oldest round at index {:?}",
                    oldest_staking_round_index
                );

                staking.remove_resolved_staking_round(oldest_staking_round_index);
            }

            current_staking_round.end_time = current_time;

            staking.add_resolved_staking_round(current_staking_round)?;
        }
    }

    // Display the total amount of collected rewards during the round
    msg!(
        "collected {} reward tokens",
        current_round_reward_token_amount
    );

    // Now that current round got resolved, setup the new current round
    {
        // Replace the current_round with the next_round
        staking.current_staking_round = StakingRound {
            start_time: current_time,
            end_time: 0,
            rate: 0,
            total_stake: staking.next_staking_round.total_stake,
            total_claim: 0,
            lm_rate: 0,
            lm_total_stake: staking.next_staking_round.lm_total_stake,
            lm_total_claim: 0,
        };
    }

    // Generate new next round
    {
        staking.next_staking_round = NextStakingRound {
            total_stake: staking.next_staking_round.total_stake,
            _padding1: Default::default(),
            lm_total_stake: staking.next_staking_round.lm_total_stake,
        };
    }

    msg!(
        "Resolved staking rounds after {:?} / {}",
        staking.resolved_staking_rounds,
        staking.resolved_staking_rounds.len()
    );
    msg!(
        "Current staking round after {:?}",
        staking.current_staking_round
    );
    msg!("Next staking round after {:?}", staking.next_staking_round);

    Ok(())
}
