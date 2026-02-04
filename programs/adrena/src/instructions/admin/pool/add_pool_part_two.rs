use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, genesis_lock::GenesisLock, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct AddPoolPartTwo<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

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
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = !pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #7
    #[account(
        init,
        payer = payer,
        space = GenesisLock::LEN,
        seeds = [b"genesis_lock", pool.key().as_ref()],
        bump
    )]
    pub genesis_lock: AccountLoader<'info, GenesisLock>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,

    /// #10
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddPoolPartTwoParams {
    // Genesis lock
    pub genesis_lock_campaign_duration: i64, // duration during which users can deposit funds
    pub genesis_reserved_grant_duration: i64, // duration during which users can claim their reserved grants (before they are transitioned to public pool)
    pub genesis_lock_campaign_start_date: i64, // start of the campaign
    // Test fields
    pub reserved_spots: ReservedSpots,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum ReservedSpots {
    None,
    #[cfg(feature = "test")]
    Test {
        first_reserved_spot: Pubkey,
        second_reserved_spot: Pubkey,
    },
}

pub fn add_pool_part_two<'info>(
    ctx: Context<'_, '_, '_, 'info, AddPoolPartTwo<'info>>,
    params: &AddPoolPartTwoParams,
) -> Result<u8> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Preliminary checks
    {
        // Genesis lock campaign is minimum 1 hour long
        if params.genesis_lock_campaign_duration < 3600 {
            msg!(
                "Genesis lock campaign duration must be at least 1 hour, {} provided",
                params.genesis_lock_campaign_duration
            );

            return Err(ProgramError::InvalidArgument.into());
        }

        // Campaign start date must be in the future
        if params.genesis_lock_campaign_start_date < ctx.accounts.cortex.load()?.get_time()? {
            msg!(
                "Genesis lock campaign start date must be in the future, {} provided / {} now",
                params.genesis_lock_campaign_start_date,
                ctx.accounts.cortex.load()?.get_time()?
            );
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    {
        pool.initialized = 1_u8;

        drop(pool);
    }

    // Record genesis_lock
    {
        let mut genesis_lock = ctx.accounts.genesis_lock.load_init()?;

        genesis_lock.bump = ctx.bumps.genesis_lock;

        genesis_lock.campaign_duration = params.genesis_lock_campaign_duration;

        genesis_lock.reserved_grant_duration = params.genesis_reserved_grant_duration;

        genesis_lock.campaign_start_date = params.genesis_lock_campaign_start_date;

        genesis_lock.has_completed_otc_out = 0;
        genesis_lock.has_completed_otc_in = 0;

        // Hardcoded genesis lock allocations
        genesis_lock.init_with_hardcoded_value()?;

        #[cfg(feature = "test")]
        {
            if let ReservedSpots::Test {
                first_reserved_spot,
                second_reserved_spot,
            } = params.reserved_spots
            {
                genesis_lock.reserved_grant_owners[10] = first_reserved_spot;
                genesis_lock.reserved_grant_owners[11] = second_reserved_spot;
            }
        }
    }

    Ok(0)
}
