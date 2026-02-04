use {
    crate::{
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex, genesis_lock::GenesisLock, pool::Pool, staking::Staking,
            user_staking::UserStaking,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct GenesisStakePatch<'info> {
    /// #1
    pub caller: Signer<'info>,

    /// #2
    // Pay for realloc
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Can be any wallet
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
        seeds = [b"pool",
                    pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #13
    #[account(
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
    pub adrena_program: Program<'info, Adrena>,

    /// #17
    pub system_program: Program<'info, System>,

    /// #18
    pub token_program: Program<'info, Token>,
}

pub fn genesis_stake_patch(_ctx: Context<GenesisStakePatch>) -> Result<()> {
    // Patched - kept for IDL and history
    //
    // let mut staking = ctx.accounts.staking.load_mut()?;
    // let mut user_staking = ctx.accounts.user_staking.load_mut()?;
    // let genesis_lock = ctx.accounts.genesis_lock.load()?;
    // let genesis_liquidity_alp_amount = ctx.accounts.cortex.load()?.genesis_liquidity_alp_amount;
    // let staking_type = staking.get_staking_type();

    // // Only for LP staking, as we target genesis ALP staking
    // {
    //     require!(
    //         staking_type == StakingType::LP,
    //         AdrenaError::InvalidStakingState
    //     );
    // }

    // // Loop over resolved rounds and:
    // // 1. Calculate rewards token amount to be claimed for staker
    // // 2. Drop fully claimed rounds
    // let (
    //     rewards_token_amount,
    //     lm_rewards_token_amount,
    //     stake_amount_with_reward_multiplier,
    //     stake_amount_with_lm_reward_multiplier,
    // ) = {
    //     let stake_token_decimals = staking.staked_token_decimals as i32;
    //     let stake_reward_token_decimals = staking.reward_token_decimals as i32;

    //     let mut rewards_token_amount: u64 = 0;
    //     let mut lm_rewards_token_amount: u64 = 0;
    //     let mut stake_amount_with_reward_multiplier: u64 = 0;
    //     let mut stake_amount_with_lm_reward_multiplier: u64 = 0;

    //     msg!(
    //         "{} resolved rounds to evaluate",
    //         staking.resolved_staking_rounds.len()
    //     );

    //     // For each resolved staking rounds
    //     let mut round_index = 0;
    //     while round_index < staking.resolved_staking_rounds.len() {
    //         let round = &mut staking.resolved_staking_rounds[round_index];

    //         if round.start_time > 0 {
    //             // Calculate rewards for locked stakes
    //             {
    //                 // For each locked stake the user has
    //                 for locked_stake in user_staking.locked_stakes.iter_mut() {
    //                     //
    //                     // Target only the problematic locked stakes
    //                     //
    //                     if genesis_lock.has_campaign_ended().unwrap()
    //                         && genesis_liquidity_alp_amount != 0
    //                         && locked_stake.is_genesis()
    //                         && locked_stake.end_time == 0
    //                     {
    //                         //
    //                         // Refund the rewards the user couldn't access due to erroneous end_time
    //                         //
    //                         if locked_stake.stake_time > 0
    //                             // Locked stake must be eligible
    //                             && locked_stake.stake_time < round.start_time
    //                             // Locked stake must not be expired
    //                             && round.end_time
    //                                 <= (locked_stake.stake_time + locked_stake.lock_duration as i64)
    //                             // Locked stake must have been claimed already
    //                             && locked_stake.claim_time >= round.start_time
    //                         {
    //                             // Stake is eligible for old rewards

    //                             // Real yield rewards
    //                             {
    //                                 let locked_rewards_token_amount = math::checked_decimal_mul(
    //                                     locked_stake.amount_with_reward_multiplier,
    //                                     -stake_token_decimals,
    //                                     round.rate,
    //                                     -(Cortex::RATE_DECIMALS as i32),
    //                                     -stake_reward_token_decimals,
    //                                 )
    //                                 .unwrap();

    //                                 rewards_token_amount += locked_rewards_token_amount;

    //                                 stake_amount_with_reward_multiplier +=
    //                                     locked_stake.amount_with_reward_multiplier;

    //                                 round.total_claim += locked_stake.amount_with_reward_multiplier;
    //                             }

    //                             // LM rewards
    //                             {
    //                                 let locked_lm_rewards_token_amount = math::checked_decimal_mul(
    //                                     locked_stake.amount_with_lm_reward_multiplier,
    //                                     -(Cortex::LM_DECIMALS as i32),
    //                                     round.lm_rate,
    //                                     -(Cortex::RATE_DECIMALS as i32),
    //                                     -(Cortex::LM_DECIMALS as i32),
    //                                 )
    //                                 .unwrap();

    //                                 lm_rewards_token_amount += locked_lm_rewards_token_amount;

    //                                 stake_amount_with_lm_reward_multiplier +=
    //                                     locked_stake.amount_with_lm_reward_multiplier;

    //                                 round.lm_total_claim +=
    //                                     locked_stake.amount_with_lm_reward_multiplier;
    //                             }
    //                         }
    //                     }
    //                 }
    //             }

    //             // retain element if there is stake that has not been claimed yet by other participants
    //             let round_fully_claimed = round.total_claim == round.total_stake
    //                 && round.lm_total_claim == round.lm_total_stake;

    //             // note: some dust of rewards will build up in the token account due to rate precision of 9 units
    //             if round_fully_claimed {
    //                 msg!("Round {} fully claimed", round_index);
    //                 staking.remove_resolved_staking_round(round_index);
    //             }
    //         }

    //         round_index += 1;
    //     }

    //     (
    //         rewards_token_amount,
    //         lm_rewards_token_amount,
    //         stake_amount_with_reward_multiplier,
    //         stake_amount_with_lm_reward_multiplier,
    //     )
    // };

    // msg!("Distribute {} rewards", rewards_token_amount);

    // {
    //     if !rewards_token_amount.is_zero() {
    //         msg!("Transfer rewards amount: {}", rewards_token_amount);

    //         let cortex = ctx.accounts.cortex.load_mut()?;

    //         cortex.transfer_tokens(
    //             ctx.accounts.staking_reward_token_vault.to_account_info(),
    //             ctx.accounts.reward_token_account.to_account_info(),
    //             ctx.accounts.transfer_authority.to_account_info(),
    //             ctx.accounts.token_program.to_account_info(),
    //             rewards_token_amount,
    //         )?;
    //     } else {
    //         msg!("No reward tokens to claim at this time");
    //     }
    // }

    // msg!("Distribute {} lm rewards", lm_rewards_token_amount);

    // {
    //     if !lm_rewards_token_amount.is_zero() {
    //         msg!(
    //             "Transfer lm_rewards_token_amount: {}",
    //             lm_rewards_token_amount
    //         );

    //         let cortex = ctx.accounts.cortex.load_mut()?;

    //         cortex.transfer_tokens(
    //             ctx.accounts.staking_lm_reward_token_vault.to_account_info(),
    //             ctx.accounts.lm_token_account.to_account_info(),
    //             ctx.accounts.transfer_authority.to_account_info(),
    //             ctx.accounts.token_program.to_account_info(),
    //             lm_rewards_token_amount,
    //         )?;
    //     } else {
    //         msg!("No lm reward tokens to claim at this time");
    //     }
    // }

    // // Update genesis stakes EndTime to unlock future claims
    // {
    //     for locked_stake in user_staking.locked_stakes.iter_mut() {
    //         if locked_stake.is_genesis() && locked_stake.end_time == 0 {
    //             locked_stake.end_time =
    //                 locked_stake.stake_time + math::checked_as_i64(locked_stake.lock_duration)?;
    //         }
    //     }
    // }

    // // Adapt resolved staked amounts to reflect the rewards claimed
    // {
    //     {
    //         // Update resolved stake token amount left, by removing the previously staked amount
    //         staking.resolved_staked_token_amount -= stake_amount_with_reward_multiplier;

    //         // Update resolved reward token amount left
    //         staking.resolved_reward_token_amount -= rewards_token_amount;
    //     }

    //     {
    //         // Update resolved lm stake token amount left, by removing the previously staked amount
    //         staking.resolved_lm_staked_token_amount -= stake_amount_with_lm_reward_multiplier;

    //         // Update resolved reward token amount left
    //         staking.resolved_lm_reward_token_amount -= lm_rewards_token_amount;
    //     }
    // }

    Ok(())
}
