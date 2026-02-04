use {
    crate::{
        adapters::{self, CreateTokenOwnerRecord, SplGovernanceV3Adapter},
        error::AdrenaError,
        math,
        program::Adrena,
        state::{
            cortex::Cortex, staking::Staking, user_staking::UserStaking, vest::Vest,
            vest_registry::VestRegistry,
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
    spl_governance::state::token_owner_record::TokenOwnerRecordV2,
    std::cmp::Ordering,
};

#[derive(Accounts)]
pub struct SyncUserVotingPower<'info> {
    /// #1
    pub caller: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Any account
    pub owner: AccountInfo<'info>,

    /// #4
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #5
    #[account(
        mut,
        seeds = [b"user_staking",
                 owner.key().as_ref(), staking.key().as_ref()],
        bump = user_staking.load()?.bump
    )]
    pub user_staking: AccountLoader<'info, UserStaking>,

    /// #6
    #[account(
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #7
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #8
    #[account(
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #9
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #10
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #11
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #12
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #13
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #14
    #[account(
        seeds = [b"vest_registry"],
        bump = vest_registry.bump,
    )]
    pub vest_registry: Box<Account<'info, VestRegistry>>,

    /// #15
    #[account(
        seeds = [b"vest", owner.key().as_ref()],
        bump = vest.load()?.bump,
        has_one = owner,
        constraint = vest.load()?.version == Vest::VERSION @AdrenaError::InvalidVestVersion
    )]
    pub vest: Option<AccountLoader<'info, Vest>>,

    /// #16
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #17
    pub adrena_program: Program<'info, Adrena>,

    /// #18
    pub system_program: Program<'info, System>,

    /// #19
    pub token_program: Program<'info, Token>,
}

// Calculate user theoretical voting power and sync it with the governance program
// Had to create a separate instruction for this because of reentrancy issue
// DAO governance execute add_vest which add governance power for the user (reentrancy error)
// Now: DAO governance execute add_vest -> sync_user_voting_power called permissionlessly
pub fn sync_user_voting_power(ctx: Context<SyncUserVotingPower>) -> Result<()> {
    let user_staking = ctx.accounts.user_staking.load()?;
    let cortex = ctx.accounts.cortex.load_mut()?;

    // Calculate voting power user should have
    let real_voting_power = {
        let mut real_voting_power = 0;

        // Staking
        {
            for locked_stake in user_staking.locked_stakes.iter() {
                if locked_stake.amount > 0 {
                    real_voting_power += math::checked_as_u64(
                        locked_stake.vote_multiplier as u128 * locked_stake.amount as u128
                            / Cortex::BPS_POWER,
                    )?;
                }
            }
        }

        // Vesting
        {
            if let Some(vest) = &ctx.accounts.vest {
                let vest = vest.load()?;

                if !vest.is_cancelled() {
                    real_voting_power +=
                        vest.apply_vote_multiplier(vest.amount - vest.claimed_amount);
                }
            } else {
                // Vest was not passed as an account
                // We need to check if the vest does actually exist and wasn't provided
                let vest_pda = Pubkey::find_program_address(
                    &[b"vest", ctx.accounts.owner.key().as_ref()],
                    ctx.program_id,
                )
                .0;

                // If the vest does exist, we can't do the sync
                if ctx
                    .accounts
                    .vest_registry
                    .vests
                    .iter()
                    .any(|p| *p == vest_pda)
                {
                    return Err(ProgramError::InvalidArgument.into());
                }
            }
        }

        real_voting_power
    };

    let current_voting_power = {
        if ctx
            .accounts
            .governance_governing_token_owner_record
            .data_is_empty()
        {
            0
        } else {
            let token_owner_record_data = ctx
                .accounts
                .governance_governing_token_owner_record
                .data
                .borrow_mut();

            let token_owner_record =
                TokenOwnerRecordV2::deserialize(&mut &token_owner_record_data[..])?;

            token_owner_record.governing_token_deposit_amount
        }
    };

    msg!("Current voting power: {}", current_voting_power);
    msg!("Real voting power: {}", real_voting_power);

    match current_voting_power.cmp(&real_voting_power) {
        Ordering::Greater => {
            // Remove excess voting power
            let excess_voting_power_amount = current_voting_power - real_voting_power;

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
                excess_voting_power_amount,
            )?;
        }
        Ordering::Less => {
            // Add missing voting power
            let missing_voting_power_amount = real_voting_power - current_voting_power;
            let mint_seeds: &[&[u8]] = &[b"governance_token_mint", &[cortex.governance_token_bump]];
            let owner_is_signer = ctx.accounts.caller.key() == ctx.accounts.owner.key();

            if !owner_is_signer
                && ctx
                    .accounts
                    .governance_governing_token_owner_record
                    .data_is_empty()
            {
                let cpi_accounts = CreateTokenOwnerRecord {
                    realm: ctx.accounts.governance_realm.to_account_info(),
                    governing_token_owner: ctx.accounts.owner.to_account_info(),
                    governing_token_owner_record: ctx
                        .accounts
                        .governance_governing_token_owner_record
                        .to_account_info(),
                    governing_token_mint: ctx.accounts.governance_token_mint.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                };

                let cpi_program = ctx.accounts.governance_program.to_account_info();
                adapters::create_token_owner_record(CpiContext::new(cpi_program, cpi_accounts))?;
            }

            cortex.add_governing_power(
                ctx.accounts.transfer_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
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
                missing_voting_power_amount,
                if owner_is_signer {
                    None
                } else {
                    Some(mint_seeds)
                },
                owner_is_signer,
            )?;
        }
        Ordering::Equal => {
            // All good
            return Ok(());
        }
    }

    Ok(())
}
