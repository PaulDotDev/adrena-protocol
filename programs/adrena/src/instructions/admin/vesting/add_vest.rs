use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            vest::{Vest, MAX_VEST_VOTE_MULTIPLIER, MIN_VEST_VOTE_MULTIPLIER},
            vest_registry::VestRegistry,
        },
        BucketName,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
pub struct AddVest<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    /// CHECK: Can be any wallet
    pub owner: AccountInfo<'info>,

    /// #3
    #[account(mut)]
    pub payer: Signer<'info>,

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
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #6
    #[account(
        mut,
        realloc = vest_registry.size() + std::mem::size_of::<Vest>(),
        realloc::payer = payer,
        realloc::zero = false,
        seeds = [b"vest_registry"],
        bump = vest_registry.bump,
    )]
    pub vest_registry: Box<Account<'info, VestRegistry>>,

    /// #7
    #[account(
        init,
        payer = payer,
        space = Vest::LEN,
        seeds = [b"vest", owner.key().as_ref()],
        bump
    )]
    pub vest: AccountLoader<'info, Vest>,

    /// #8
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #15
    pub system_program: Program<'info, System>,

    /// #16
    pub token_program: Program<'info, Token>,

    /// #17
    pub rent: Sysvar<'info, Rent>,
}

const SEVEN_DAYS_IN_SECONDS: i64 = 3_600 * 24 * 7;

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct AddVestParams {
    pub amount: u64,
    pub origin_bucket: u8, // BucketName
    pub unlock_start_timestamp: i64,
    pub unlock_end_timestamp: i64,
    pub vote_multiplier: u32, // in BPS
}

pub fn add_vest<'info>(
    ctx: Context<'_, '_, '_, 'info, AddVest<'info>>,
    params: &AddVestParams,
) -> Result<u8> {
    let mut vest = ctx.accounts.vest.load_init()?;

    let current_time = ctx.accounts.cortex.load()?.get_time()?;

    // validate inputs
    {
        if params.amount == 0 || params.unlock_end_timestamp <= params.unlock_start_timestamp {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Unlock must end in minimum 7 days
        require!(
            params.unlock_end_timestamp >= (current_time + SEVEN_DAYS_IN_SECONDS),
            AdrenaError::InvalidVestingUnlockTime
        );

        // Vesting must be at least 7 days long
        require!(
            (params.unlock_end_timestamp - params.unlock_start_timestamp) >= SEVEN_DAYS_IN_SECONDS,
            AdrenaError::InvalidVestingUnlockTime
        );

        // Gate keeper
        require!(
            // x4 MAX
            params.vote_multiplier <= MAX_VEST_VOTE_MULTIPLIER,
            AdrenaError::InvalidVoteMultiplier
        );

        require!(
            // x1 MIN
            params.vote_multiplier >= MIN_VEST_VOTE_MULTIPLIER,
            AdrenaError::InvalidVoteMultiplier
        );
    }

    let origin_bucket = BucketName::try_from(params.origin_bucket)?;

    // setup vest account
    {
        if vest.amount != 0 && vest.claimed_amount < vest.amount {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }

        msg!(
            "Record vest: amount {}, owner {}, unlock_start_timestamp {}, unlock_end_timestamp: {}",
            params.amount,
            ctx.accounts.owner.key,
            params.unlock_start_timestamp,
            params.unlock_end_timestamp,
        );

        vest.amount = params.amount;
        vest.origin_bucket = origin_bucket.into();
        vest.unlock_start_timestamp = params.unlock_start_timestamp;
        vest.unlock_end_timestamp = params.unlock_end_timestamp;
        vest.claimed_amount = 0;
        vest.last_claim_timestamp = 0;
        vest.owner = ctx.accounts.owner.key();
        vest.bump = ctx.bumps.vest;
        vest.vote_multiplier = params.vote_multiplier;
        vest.cancelled = false as u8;
        vest.has_delegate = false as u8;
        vest.delegate = Pubkey::default();
        vest.version = Vest::VERSION;
    }

    // Add vest to registry
    {
        let vest_registry = ctx.accounts.vest_registry.as_mut();

        vest_registry.vests.push(ctx.accounts.vest.key());
        vest_registry.vesting_token_amount += params.amount;

        let mut cortex = ctx.accounts.cortex.load_mut()?;

        // update reserved amount in bucket
        cortex.update_bucket_vested_amount(
            origin_bucket,
            i64::try_from(params.amount).map_err(|_| AdrenaError::MathOverflow)?,
        );

        drop(cortex);
    }

    Ok(0)
}
