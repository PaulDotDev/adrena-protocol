use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        events::RemoveLockedStakeEvent,
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
pub struct RemoveLockedStake<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        token::mint = lm_token_mint,
        has_one = owner
    )]
    pub lm_token_account: Box<Account<'info, TokenAccount>>,

    /// #3
    #[account(
        mut,
        token::mint = staked_token_mint,
        has_one = owner
    )]
    pub staked_token_account: Box<Account<'info, TokenAccount>>,

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
        bump = user_staking.load()?.bump,
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #10
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        has_one = staked_token_mint,
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
    #[account(mut)]
    pub staked_token_mint: Box<Account<'info, Mint>>,

    /// #16
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #17
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #18
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #19
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #20
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #21
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #22
    governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #23
    adrena_program: Program<'info, Adrena>,

    /// #24
    system_program: Program<'info, System>,

    /// #25
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct RemoveLockedStakeParams {
    pub locked_stake_index: u64,
}

// Remove one stake at a time
pub fn remove_locked_stake(
    ctx: Context<RemoveLockedStake>,
    params: &RemoveLockedStakeParams,
) -> Result<()> {
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

    let mut user_staking = ctx.accounts.user_staking.load_mut()?;

    let (token_amount_to_unstake, token_amount_to_burn, locked_stake_id) = {
        let locked_stake = user_staking
            .locked_stakes
            .get(params.locked_stake_index as usize)
            .ok_or(AdrenaError::UserStakeNotFound)?;

        let locked_stake_id = locked_stake.id;

        // Check the stake have ended and have been resolved
        {
            let current_time = ctx.accounts.cortex.load()?.get_time()?;

            require!(
                (locked_stake.is_early_exit() || locked_stake.has_ended(current_time)?)
                    && locked_stake.is_resolved(),
                AdrenaError::UnresolvedStake
            );
        }

        let token_amount_to_unstake = locked_stake.amount - locked_stake.early_exit_fee;
        let token_amount_to_burn = locked_stake.early_exit_fee;

        // Remove the stake from the list
        user_staking.remove_locked_stake(params.locked_stake_index as usize)?;

        (
            token_amount_to_unstake,
            token_amount_to_burn,
            locked_stake_id,
        )
    };

    // Unstake owner's tokens
    {
        let cortex = ctx.accounts.cortex.load_mut()?;

        cortex.transfer_tokens(
            ctx.accounts.staking_staked_token_vault.to_account_info(),
            ctx.accounts.staked_token_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token_amount_to_unstake,
        )?;
    }

    if token_amount_to_burn > 0 {
        msg!("Burn early un-stake tax: {}", token_amount_to_burn);

        let cortex = ctx.accounts.cortex.load_mut()?;

        cortex.burn_tokens(
            ctx.accounts.staked_token_mint.to_account_info(),
            ctx.accounts.staking_staked_token_vault.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token_amount_to_burn,
        )?;
    }

    emit!(RemoveLockedStakeEvent {
        owner: ctx.accounts.owner.key(),
        staking: ctx.accounts.staking.key(),
        locked_stake_id,
    });

    Ok(())
}
