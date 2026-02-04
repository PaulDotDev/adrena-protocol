use {
    crate::{
        error::AdrenaError,
        events::AddLockedStakeEvent,
        math,
        program::Adrena,
        state::{
            cortex::Cortex,
            staking::{Staking, StakingType},
            user_staking::{LockedStake, UserStaking},
        },
        MintLmTokensFromBucketParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct MintStakedLmTokensFromBucket<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Any account
    pub owner: AccountInfo<'info>,

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
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #6
    #[account(
        mut,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

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
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
        has_one = admin
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
    pub adrena_program: Program<'info, Adrena>,

    /// #11
    pub system_program: Program<'info, System>,

    /// #12
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintStakedLmTokensFromBucketParams {
    // BucketName Enum
    pub bucket_name: u8,
    pub amount: u64,
    pub reason: String,
    // Amount of days to be locked for
    pub locked_days: u32,
}

pub fn mint_staked_lm_tokens_from_bucket<'info>(
    ctx: Context<'_, '_, '_, 'info, MintStakedLmTokensFromBucket<'info>>,
    params: &MintStakedLmTokensFromBucketParams,
) -> Result<()> {
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
        // Only authorize LM staking
        if staking.get_staking_type() != StakingType::LM {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Set the staking type if not already set - this can happens for stakes created before the staking type was introduced
        if user_staking.staking_type == 0 {
            user_staking.staking_type = staking.staking_type;
        }
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

    {
        let transfer_authority_bump = cortex.transfer_authority_bump;

        drop(cortex);

        // Mint LM tokens
        {
            // Mint directly to LM staking token vault
            let cpi_accounts = crate::cpi::accounts::MintLmTokensFromBucket {
                admin: ctx.accounts.transfer_authority.to_account_info(),
                receiving_account: ctx.accounts.staking_staked_token_vault.to_account_info(),
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
                    bucket_name: params.bucket_name,
                    amount: params.amount,
                    reason: params.reason.clone(),
                },
            )?;

            // Reload account to account for newly minted tokens
            {
                ctx.accounts.staking_staked_token_vault.reload()?;
                ctx.accounts.lm_token_mint.reload()?;
            }
        }
    }

    staking.next_staking_round.total_stake += stake_amount_with_reward_multiplier;

    staking.next_staking_round.lm_total_stake += stake_amount_with_lm_reward_multiplier;

    staking.nb_locked_tokens += params.amount;

    emit!(AddLockedStakeEvent {
        owner: ctx.accounts.owner.key(),
        staking: ctx.accounts.staking.key(),
        locked_stake_id,
        amount: params.amount,
        locked_days: params.locked_days,
    });

    Ok(())
}
