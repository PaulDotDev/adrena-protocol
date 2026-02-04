use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            custody::Custody,
            genesis_lock::GenesisLock,
            pool::{Pool, PoolLiquidityState},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct GenesisOtcIn<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        constraint = funding_account_one.mint == custody_one.load()?.mint,
        constraint = funding_account_one.owner == *admin.key
    )]
    pub funding_account_one: Box<Account<'info, TokenAccount>>,

    /// #4
    #[account(
        mut,
        constraint = funding_account_two.mint == custody_two.load()?.mint,
        constraint = funding_account_two.owner == *admin.key
    )]
    pub funding_account_two: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        constraint = funding_account_three.mint == custody_three.load()?.mint,
        constraint = funding_account_three.owner == *admin.key
    )]
    pub funding_account_three: Box<Account<'info, TokenAccount>>,

    /// #6
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        seeds = [b"pool",
                pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #8
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_one.load()?.mint.as_ref()],
        bump = custody_one.load()?.bump
    )]
    pub custody_one: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_one.load()?.mint.as_ref()],
        bump = custody_one.load()?.token_account_bump
    )]
    pub custody_one_token_account: Box<Account<'info, TokenAccount>>,

    /// #10
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_two.load()?.mint.as_ref()],
        bump = custody_two.load()?.bump
    )]
    pub custody_two: AccountLoader<'info, Custody>,

    /// #11
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_two.load()?.mint.as_ref()],
        bump = custody_two.load()?.token_account_bump
    )]
    pub custody_two_token_account: Box<Account<'info, TokenAccount>>,

    /// #12
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_three.load()?.mint.as_ref()],
        bump = custody_three.load()?.bump
    )]
    pub custody_three: AccountLoader<'info, Custody>,

    /// #13
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_three.load()?.mint.as_ref()],
        bump = custody_three.load()?.token_account_bump
    )]
    pub custody_three_token_account: Box<Account<'info, TokenAccount>>,

    /// #14
    #[account(
        mut,
        seeds = [b"genesis_lock", pool.key().as_ref()],
        bump = genesis_lock.load()?.bump
    )]
    pub genesis_lock: AccountLoader<'info, GenesisLock>,

    /// #15
    pub system_program: Program<'info, System>,

    /// #16
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GenesisOtcInParams {
    pub custody_one_amount: u64,
    pub custody_two_amount: u64,
    pub custody_three_amount: u64,
}

pub fn genesis_otc_in(ctx: Context<GenesisOtcIn>, params: &GenesisOtcInParams) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    let cortex = ctx.accounts.cortex.load()?;
    let mut genesis_lock = ctx.accounts.genesis_lock.load_mut()?;

    // Check that custodies are the right one
    {
        // custody[0] is USDC by design - and USDC is not swapped OTC
        if ctx.accounts.custody_one.key() != pool.custodies[1]
            || ctx.accounts.custody_two.key() != pool.custodies[2]
            || ctx.accounts.custody_three.key() != pool.custodies[3]
        {
            return Err(AdrenaError::InvalidCustody.into());
        }
    }

    {
        require!(
            !genesis_lock.is_campaign_open()?,
            AdrenaError::InstructionNotAllowed
        );

        // OUT must be done and IN must not be done
        require!(
            genesis_lock.is_otc_out_completed(),
            AdrenaError::InstructionNotAllowed
        );

        require!(
            !genesis_lock.is_otc_in_completed(),
            AdrenaError::InstructionNotAllowed
        );
    }

    {
        require!(
            pool.get_liquidity_state().eq(&PoolLiquidityState::Idle),
            AdrenaError::InstructionNotAllowed
        );
    }

    // Transfers tokens from user to the pool
    {
        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account_one.to_account_info(),
            ctx.accounts.custody_one_token_account.to_account_info(),
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.custody_one_amount,
        )?;

        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account_two.to_account_info(),
            ctx.accounts.custody_two_token_account.to_account_info(),
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.custody_two_amount,
        )?;

        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account_three.to_account_info(),
            ctx.accounts.custody_three_token_account.to_account_info(),
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            params.custody_three_amount,
        )?;
    }

    // Take into account new pool assets
    {
        {
            let mut custody_one = ctx.accounts.custody_one.load_mut()?;
            custody_one.assets.owned += params.custody_one_amount;
        }
        {
            let mut custody_two = ctx.accounts.custody_two.load_mut()?;
            custody_two.assets.owned += params.custody_two_amount;
        }
        {
            let mut custody_three = ctx.accounts.custody_three.load_mut()?;
            custody_three.assets.owned += params.custody_three_amount;
        }
    }

    // All done
    genesis_lock.has_completed_otc_in = 1;

    Ok(())
}
