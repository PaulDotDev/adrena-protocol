use {
    crate::{
        error::AdrenaError,
        instructions::BucketName,
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            genesis_lock::GenesisLock,
            pool::Pool,
            staking::{Staking, StakingType},
            user_staking::UserStaking,
        },
        MintLmTokensFromBucketParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    num::Zero,
};

#[derive(Accounts)]
pub struct ClaimStakes<'info> {
    /// #1
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #2
    // Pay for realloc
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Can be any wallet
    #[account(mut)]
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
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #9
    #[account(
        mut,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #10
    #[account(
        mut,
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
    adrena_program: Program<'info, Adrena>,

    /// #17
    system_program: Program<'info, System>,

    /// #18
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ClaimStakesParams {
    pub locked_stake_indexes: Option<Vec<u8>>,
}

pub fn claim_stakes(ctx: Context<ClaimStakes>, params: &ClaimStakesParams) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;
    let mut user_staking = ctx.accounts.user_staking.load_mut()?;
    let genesis_lock = ctx.accounts.genesis_lock.load()?;
    let transfer_authority_bump = ctx.accounts.cortex.load()?.transfer_authority_bump;
    let genesis_liquidity_alp_amount = ctx.accounts.cortex.load()?.genesis_liquidity_alp_amount;
    let staking_type = staking.get_staking_type();

    // Set the staking type if not already set - this can happens for stakes created before the staking type was introduced
    if user_staking.staking_type == 0 {
        user_staking.staking_type = staking.staking_type;
    }

    if let Some(locked_stake_indexes) = &params.locked_stake_indexes {
        for index in locked_stake_indexes {
            if *index as usize >= user_staking.locked_stakes.len() {
                return Err(ProgramError::InvalidArgument.into());
            }
        }

        // Have an empty locked_stake_indexes array is ok, we just claim for liquids
    }

    // Loop over resolved rounds and:
    // 1. Calculate rewards token amount to be claimed for staker
    // 2. Drop fully claimed rounds
    let (
        rewards_token_amount,
        stake_amount_with_reward_multiplier,
        lm_rewards_token_amount,
        stake_amount_with_lm_reward_multiplier,
    ) = {
        let stake_token_decimals = staking.staked_token_decimals as i32;
        let stake_reward_token_decimals = staking.reward_token_decimals as i32;

        let mut rewards_token_amount: u64 = 0;
        let mut lm_rewards_token_amount: u64 = 0;
        let mut lm_extra_rewards_token_amount: u64 = 0;

        // total amount of token that tokens have been claimed for
        let mut stake_amount_with_reward_multiplier: u64 = 0;
        let mut stake_amount_with_lm_reward_multiplier: u64 = 0;

        msg!(
            "{} resolved rounds to evaluate",
            staking.resolved_staking_rounds.len()
        );

        {
            if genesis_lock.has_campaign_ended().unwrap() && genesis_liquidity_alp_amount != 0 {
                msg!("Genesis lock extra rewards calculation and distribution");

                let reward_distribution_end_date =
                    genesis_lock.reward_distribution_period_end_date();

                // For each user locked stakes that is not a placeholder in the array
                for locked_stake in user_staking
                    .locked_stakes
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(index, stake)| {
                        if params
                            .locked_stake_indexes
                            .as_ref()
                            .map_or(true, |indexes| indexes.contains(&(index as u8)))
                        {
                            Some(stake)
                        } else {
                            None
                        }
                    })
                    .filter(|stake| stake.amount != 0)
                {
                    // Genesis lock extra rewards
                    // Make sure that the user didn't already claim the rewards for the genesis lock
                    // (genesis_claim_time is updated after the rewards are claimed, up to the reward_distribution_end_date of the campaign)
                    if locked_stake.is_genesis()
                        && locked_stake.genesis_claim_time < reward_distribution_end_date
                    {
                        let current_time = GenesisLock::get_time().unwrap();

                        let genesis_reward_rate = GenesisLock::reward_rate_per_alp_per_second(
                            genesis_liquidity_alp_amount,
                        );

                        if current_time > locked_stake.genesis_claim_time {
                            // Find out the duration since last claim, and clamp it to the max amount if the user claims after the campaign ends
                            let max_period =
                                reward_distribution_end_date - locked_stake.genesis_claim_time;
                            let period = current_time - locked_stake.genesis_claim_time;
                            let clamped_period = std::cmp::min(period, max_period);

                            let genesis_rewards = math::checked_as_u64(
                                locked_stake.amount as u128
                                    * clamped_period as u128
                                    * genesis_reward_rate
                                    / Cortex::RATE_POWER,
                            )
                            .unwrap();

                            lm_extra_rewards_token_amount += genesis_rewards;

                            locked_stake.genesis_claim_time = current_time;
                        }
                    }
                }

                // Mint LM tokens extra rewards
                {
                    if lm_extra_rewards_token_amount > 0 {
                        let cpi_accounts = crate::cpi::accounts::MintLmTokensFromBucket {
                            admin: ctx.accounts.transfer_authority.to_account_info(),
                            receiving_account: ctx.accounts.lm_token_account.to_account_info(),
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
                                amount: lm_extra_rewards_token_amount,
                                reason: String::from("Genesis lock campaign rewards"),
                            },
                        )?;

                        {
                            ctx.accounts.staking_lm_reward_token_vault.reload()?;
                            ctx.accounts.lm_token_mint.reload()?;
                        }
                    }
                }
            }
        }

        // For each resolved staking rounds
        let mut round_index = 0;
        while round_index < staking.resolved_staking_rounds.len() {
            let round: &mut crate::state::staking::StakingRound =
                &mut staking.resolved_staking_rounds[round_index];

            if round.start_time > 0 {
                // Calculate rewards for locked stakes
                {
                    // For each user locked stakes
                    for locked_stake in user_staking
                        .locked_stakes
                        .iter_mut()
                        .enumerate()
                        .filter_map(|(index, stake)| {
                            if params
                                .locked_stake_indexes
                                .as_ref()
                                .map_or(true, |indexes| indexes.contains(&(index as u8)))
                            {
                                Some(stake)
                            } else {
                                None
                            }
                        })
                    {
                        // Stake is eligible for rewards
                        if locked_stake.qualifies_for_rewards_from(round) {
                            {
                                // Increment the qualified_for_rewards_in_resolved_rounds_count (Established related)
                                locked_stake.qualified_for_rewards_in_resolved_round_count += 1;

                                let locked_rewards_token_amount = math::checked_decimal_mul(
                                    locked_stake.amount_with_reward_multiplier,
                                    -stake_token_decimals,
                                    round.rate,
                                    -(Cortex::RATE_DECIMALS as i32),
                                    -stake_reward_token_decimals,
                                )
                                .unwrap();

                                rewards_token_amount += locked_rewards_token_amount;

                                round.total_claim += locked_stake.amount_with_reward_multiplier;

                                stake_amount_with_reward_multiplier +=
                                    locked_stake.amount_with_reward_multiplier;
                            }

                            {
                                let locked_lm_rewards_token_amount = math::checked_decimal_mul(
                                    locked_stake.amount_with_lm_reward_multiplier,
                                    -(Cortex::LM_DECIMALS as i32),
                                    round.lm_rate,
                                    -(Cortex::RATE_DECIMALS as i32),
                                    -(Cortex::LM_DECIMALS as i32),
                                )
                                .unwrap();

                                lm_rewards_token_amount += locked_lm_rewards_token_amount;

                                round.lm_total_claim +=
                                    locked_stake.amount_with_lm_reward_multiplier;

                                stake_amount_with_lm_reward_multiplier +=
                                    locked_stake.amount_with_lm_reward_multiplier;
                            }
                        }
                    }
                }

                // Calculate rewards for liquid stake
                {
                    // Stake is eligible for rewards
                    if user_staking.liquid_stake.qualifies_for_rewards_from(round) {
                        let stake_amount = if user_staking.liquid_stake.overlap_amount > 0
                            && user_staking.liquid_stake.overlap_time >= round.start_time
                        {
                            // there is an overlap, which means that current_round staked amount is different that
                            // the staked amount in the resolved round
                            //
                            // it is the case because user staked tokens when they were already staked token
                            //
                            // i.e
                            // initial context: resolved round [], current round [10 staked tokens] next round [10 staked tokens]
                            // user stakes 20 new tokens: resolved round [], current round [10 staked tokens] next round [30 staked tokens]
                            // after round resolve: resolved round [10 stake_amount], current round [30 staked tokens] next round [30 staked tokens]
                            //
                            // now we are in the case where claim is called for the resolved round. User is not entitled to 30 stake tokens worth or rewards
                            // but 10
                            let stake_amount = user_staking.liquid_stake.amount
                                - user_staking.liquid_stake.overlap_amount;

                            msg!("Overlap");

                            user_staking.liquid_stake.overlap_amount = 0;
                            user_staking.liquid_stake.overlap_time = 0;

                            stake_amount
                        } else {
                            user_staking.liquid_stake.amount
                        };

                        if staking_type == StakingType::LM {
                            let liquid_rewards_token_amount = math::checked_decimal_mul(
                                stake_amount,
                                -stake_token_decimals,
                                round.rate,
                                -(Cortex::RATE_DECIMALS as i32),
                                -stake_reward_token_decimals,
                            )
                            .unwrap();

                            rewards_token_amount += liquid_rewards_token_amount;

                            round.total_claim += stake_amount;

                            stake_amount_with_reward_multiplier += stake_amount;
                        }

                        if staking_type == StakingType::LP {
                            return Err(AdrenaError::InvalidStakingState.into());
                        }
                    }
                }

                // retain element if there is stake that has not been claimed yet by other participants
                let round_fully_claimed = round.total_claim == round.total_stake
                    && round.lm_total_claim == round.lm_total_stake;

                // note: some dust of rewards will build up in the token account due to rate precision of 9 units
                if round_fully_claimed {
                    staking.remove_resolved_staking_round(round_index);
                }
            }

            round_index += 1;
        }

        (
            rewards_token_amount,
            stake_amount_with_reward_multiplier,
            lm_rewards_token_amount,
            stake_amount_with_lm_reward_multiplier,
        )
    };

    msg!("Distribute {} rewards", rewards_token_amount);

    {
        if !rewards_token_amount.is_zero() {
            msg!("Transfer rewards amount: {}", rewards_token_amount);

            let cortex = ctx.accounts.cortex.load_mut()?;

            cortex.transfer_tokens(
                ctx.accounts.staking_reward_token_vault.to_account_info(),
                ctx.accounts.reward_token_account.to_account_info(),
                ctx.accounts.transfer_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                rewards_token_amount,
            )?;
        } else {
            msg!("No reward tokens to claim at this time");
        }
    }

    msg!("Distribute {} lm rewards", lm_rewards_token_amount);

    {
        if !lm_rewards_token_amount.is_zero() {
            msg!(
                "Transfer lm_rewards_token_amount: {}",
                lm_rewards_token_amount
            );

            let cortex = ctx.accounts.cortex.load_mut()?;

            cortex.transfer_tokens(
                ctx.accounts.staking_lm_reward_token_vault.to_account_info(),
                ctx.accounts.lm_token_account.to_account_info(),
                ctx.accounts.transfer_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                lm_rewards_token_amount,
            )?;
        } else {
            msg!("No lm reward tokens to claim at this time");
        }
    }

    // Update stakes claim time
    {
        // refresh claim time while keeping the claim time out of the current round
        // so that the user stay eligible for current round rewards
        let claim_time = staking.current_staking_round.start_time - 1;

        if let Some(indexes) = &params.locked_stake_indexes {
            for index in indexes {
                user_staking.locked_stakes[*index as usize].claim_time = claim_time;
            }
        } else {
            // For each user locked stakes
            for locked_stake in user_staking.locked_stakes.iter_mut() {
                locked_stake.claim_time = claim_time;
            }
        }

        // Liquid staking
        user_staking.liquid_stake.claim_time = claim_time;
    }

    // Adapt current/next round
    {
        {
            // Update resolved stake token amount left, by removing the previously staked amount
            staking.resolved_staked_token_amount -= stake_amount_with_reward_multiplier;

            // Update resolved reward token amount left
            staking.resolved_reward_token_amount -= rewards_token_amount;
        }

        {
            // Update resolved lm stake token amount left, by removing the previously staked amount
            staking.resolved_lm_staked_token_amount -= stake_amount_with_lm_reward_multiplier;

            // Update resolved reward token amount left
            staking.resolved_lm_reward_token_amount -= lm_rewards_token_amount;
        }

        msg!(
            "Resolved reward token amount after claim stake {:?}",
            staking.resolved_reward_token_amount
        );

        msg!(
            "Resolved lm reward token amount after claim stake {:?}",
            staking.resolved_lm_reward_token_amount
        );

        // msg!(
        //     "Resolved staking rounds after claim stake {:?}",
        //     staking.resolved_staking_rounds
        // );
    }

    Ok(())
}
