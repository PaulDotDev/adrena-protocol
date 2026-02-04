use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
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

#[derive(Accounts)]
pub struct RemoveLiquidStake<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        token::mint = staking.load()?.staked_token_mint,
        has_one = owner
    )]
    pub staked_token_account: Box<Account<'info, TokenAccount>>,

    /// #3
    #[account(
        mut,
        token::mint = lm_token_mint,
        has_one = owner
    )]
    pub lm_token_account: Box<Account<'info, TokenAccount>>,

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
        token::mint = staking.load()?.staked_token_mint,
        token::authority = transfer_authority,
        seeds = [b"staking_staked_token_vault", staking.key().as_ref()],
        bump = staking.load()?.staked_token_vault_bump
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
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
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
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #16
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #17
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #18
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #19
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #20
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #21
    governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #22
    adrena_program: Program<'info, Adrena>,

    /// #23
    system_program: Program<'info, System>,

    /// #24
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct RemoveLiquidStakeParams {
    pub amount: u64,
}

pub fn remove_liquid_stake(
    ctx: Context<RemoveLiquidStake>,
    params: &RemoveLiquidStakeParams,
) -> Result<()> {
    // Preliminary checks
    {
        if params.amount == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    // Claim existing rewards before removing the stake
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

    {
        // Verify user staked balance
        require!(
            user_staking.liquid_stake.amount >= params.amount,
            AdrenaError::InvalidStakeState
        );

        // Revoke 1:1 governing power allocated to the stake
        if staking.get_staking_type() == StakingType::LM {
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
                params.amount,
            )?;
        }

        // Apply delta to user stake
        user_staking.liquid_stake.amount -= params.amount;

        // Apply delta to staking
        staking.nb_liquid_tokens -= params.amount;

        // Apply delta to current and next round
        {
            // Forfeit current round participation, if any
            if user_staking
                .liquid_stake
                .qualifies_for_rewards_from(&staking.current_staking_round)
            {
                // Overlap
                if user_staking.liquid_stake.overlap_amount > 0
                    && user_staking.liquid_stake.overlap_time
                        >= staking.current_staking_round.start_time
                {
                    // In case of overlap, takes overlapped tokens first (last tokens put in staking)
                    //
                    // if there are not enough tokens, takes it up from long lasting staked tokens reserve
                    if params.amount > user_staking.liquid_stake.overlap_amount {
                        // Remove the non-overlapped tokens from current round stake
                        staking.current_staking_round.total_stake -=
                            params.amount - user_staking.liquid_stake.overlap_amount;

                        user_staking.liquid_stake.overlap_amount = 0;
                    } else {
                        user_staking.liquid_stake.overlap_amount -= params.amount;
                    }
                } else {
                    staking.current_staking_round.total_stake -= params.amount;
                }
            }

            // Apply delta to next round
            staking.next_staking_round.total_stake -= params.amount;
        }
    };

    // Un-stake owner's tokens
    {
        cortex.transfer_tokens(
            ctx.accounts.staking_staked_token_vault.to_account_info(),
            ctx.accounts.staked_token_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.amount,
        )?;
    }

    Ok(())
}
