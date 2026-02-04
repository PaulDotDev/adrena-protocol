use {
    crate::{
        adapters::SplGovernanceV3Adapter,
        error::AdrenaError,
        instructions::MintLmTokensFromBucketParams,
        program::Adrena,
        state::{cortex::Cortex, vest::Vest, vest_registry::VestRegistry},
        BucketName,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
#[instruction()]
pub struct ClaimVest<'info> {
    /// #1
    /// CHECK: Check the caller in the instruction
    pub caller: Signer<'info>,

    /// #2
    /// CHECK: Checked to be the wallet related to the vest
    pub owner: AccountInfo<'info>,

    /// #3
    /// CHECK: Any account
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #4
    /// CHECK: Ownership checked in the instruction
    #[account(
        mut,
        constraint = receiving_account.mint == lm_token_mint.key(),
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

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
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = governance_program @AdrenaError::InvalidGovernanceProgram,
        has_one = governance_realm @AdrenaError::InvalidGovernanceRealm,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        mut,
        seeds = [b"vest_registry"],
        bump = vest_registry.bump,
    )]
    pub vest_registry: Box<Account<'info, VestRegistry>>,

    /// #8
    #[account(
        mut,
        seeds = [b"vest", owner.key().as_ref()],
        bump = vest.load()?.bump,
        has_one = owner,
        constraint = !vest.load()?.is_cancelled() @AdrenaError::InvalidVestState,
        constraint = vest.load()?.version == Vest::VERSION @AdrenaError::InvalidVestVersion,
    )]
    pub vest: AccountLoader<'info, Vest>,

    /// #9
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #10
    #[account(
        mut,
        seeds = [b"governance_token_mint"],
        bump = cortex.load()?.governance_token_bump
    )]
    pub governance_token_mint: Box<Account<'info, Mint>>,

    /// #11
    /// CHECK: Checked by spl governance v3 program
    /// A realm represent one project within the governance program
    pub governance_realm: UncheckedAccount<'info>,

    /// #12
    /// CHECK: Checked by spl governance v3 program
    pub governance_realm_config: UncheckedAccount<'info>,

    /// #13
    /// CHECK: Checked by spl governance v3 program
    /// Token account owned by governance program holding user's locked tokens
    #[account(mut)]
    pub governance_governing_token_holding: UncheckedAccount<'info>,

    /// #14
    /// CHECK: Checked by spl governance v3 program
    /// Account owned by governance storing user information
    #[account(mut)]
    pub governance_governing_token_owner_record: UncheckedAccount<'info>,

    /// #15
    pub governance_program: Program<'info, SplGovernanceV3Adapter>,

    /// #16
    pub adrena_program: Program<'info, Adrena>,

    /// #17
    pub system_program: Program<'info, System>,

    /// #18
    pub token_program: Program<'info, Token>,

    /// #19
    pub rent: Sysvar<'info, Rent>,
}

// Return claimed amount
pub fn claim_vest<'info>(ctx: Context<'_, '_, '_, 'info, ClaimVest<'info>>) -> Result<u64> {
    let mut vest = ctx.accounts.vest.load_mut()?;
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    // Check the CALLER
    {
        let caller = ctx.accounts.caller.key();

        if caller != ctx.accounts.owner.key()
            && caller != cortex.admin
            && (!vest.has_delegate() || caller != vest.delegate)
        {
            return Err(AdrenaError::InvalidCaller.into());
        }
    }

    // Check the owner of the token account
    {
        let token_account_owner = ctx.accounts.receiving_account.owner;

        if token_account_owner != vest.owner
            && (!vest.has_delegate() || token_account_owner != vest.delegate)
        {
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    let current_time = cortex.get_time()?;
    let transfer_authority_bump = cortex.transfer_authority_bump;

    let claimable_amount = vest.get_claimable_amount(current_time)?;

    if claimable_amount == 0 {
        return Ok(0);
    }

    drop(cortex);

    // Mint lm token to user account
    if claimable_amount > 0 {
        let cpi_accounts = crate::cpi::accounts::MintLmTokensFromBucket {
            admin: ctx.accounts.transfer_authority.to_account_info(),
            receiving_account: ctx.accounts.receiving_account.to_account_info(),
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
                bucket_name: vest.origin_bucket,
                amount: claimable_amount,
                reason: String::from("Liquidity mining rewards"),
            },
        )?;

        {
            ctx.accounts.receiving_account.reload()?;
            ctx.accounts.lm_token_mint.reload()?;
        }
    }

    // Update vest accounting
    {
        vest.claimed_amount += claimable_amount;
        vest.last_claim_timestamp = current_time;
    }

    let vest_registry = ctx.accounts.vest_registry.as_mut();

    cortex = ctx.accounts.cortex.load_mut()?;

    // Accounting
    {
        vest_registry.vesting_token_amount -= claimable_amount;
        vest_registry.vested_token_amount += claimable_amount;

        // Subtract the claimed amount from the bucket reserved amount
        cortex.update_bucket_vested_amount(
            BucketName::try_from(vest.origin_bucket)?,
            -(i64::try_from(claimable_amount).map_err(|_| AdrenaError::MathOverflow)?),
        );
    }

    // If everything have been claimed, remove vesting from the registry
    if vest.claimed_amount == vest.amount {
        let vest_idx = vest_registry
            .vests
            .iter()
            .position(|x| *x == ctx.accounts.vest.key())
            .ok_or(AdrenaError::InvalidVestState)?;

        vest_registry.vests.remove(vest_idx);

        msg!("Realloc vest_registry size to accommodate for the removed vest");

        // Note: the vest PDA still lives, we can un-allocate (currently works same as Pool, without removal)
        {
            Cortex::realloc(
                ctx.accounts.payer.to_account_info(),
                vest_registry.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                vest_registry.size(),
                true,
            )?;
        }
    }

    // Revoke 1:X governing power for each claimed tokens
    let voting_power_amount = vest.apply_vote_multiplier(claimable_amount);

    if voting_power_amount > 0 {
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
            voting_power_amount,
        )?;
    }

    Ok(claimable_amount)
}
