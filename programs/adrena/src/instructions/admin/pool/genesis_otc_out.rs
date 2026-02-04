use {
    crate::{
        error::AdrenaError,
        math,
        state::{cortex::Cortex, custody::Custody, genesis_lock::GenesisLock, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct GenesisOtcOut<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        constraint = dao_receiving_account.mint == custody_usdc.load()?.mint,
    )]
    pub dao_receiving_account: Box<Account<'info, TokenAccount>>,

    /// #4
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #5
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
        has_one = admin,
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #6
    #[account(
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #7
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_usdc.load()?.mint.as_ref()],
        bump = custody_usdc.load()?.bump
    )]
    pub custody_usdc: AccountLoader<'info, Custody>,

    /// #8
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_usdc.load()?.mint.as_ref()],
        bump = custody_usdc.load()?.token_account_bump
    )]
    pub custody_usdc_token_account: Box<Account<'info, TokenAccount>>,

    /// #9
    #[account(
        mut,
        seeds = [b"genesis_lock", pool.key().as_ref()],
        bump = genesis_lock.load()?.bump
    )]
    pub genesis_lock: AccountLoader<'info, GenesisLock>,

    /// #10
    pub token_program: Program<'info, Token>,
}

pub fn genesis_otc_out(ctx: Context<GenesisOtcOut>) -> Result<()> {
    let cortex = ctx.accounts.cortex.load()?;
    let mut custody = ctx.accounts.custody_usdc.load_mut()?;
    let mut genesis_lock = ctx.accounts.genesis_lock.load_mut()?;

    // CHECK - the genesis lock campaign is over
    require!(
        !genesis_lock.is_campaign_open()?,
        AdrenaError::InstructionNotAllowed
    );

    // OUT should not be completed yet, neither IN
    {
        require!(
            !genesis_lock.is_otc_out_completed(),
            AdrenaError::InstructionNotAllowed
        );

        require!(
            !genesis_lock.is_otc_in_completed(),
            AdrenaError::InstructionNotAllowed
        );
    }

    // Transfer 70% of the USDC out of the pool
    let usdc_amount_out = {
        let total_usdc_raised =
            genesis_lock.public_amount_claimed + genesis_lock.reserved_amount_claimed;

        math::checked_as_u64((total_usdc_raised as u128 * 70_u128) / 100_u128)?
    };

    cortex.transfer_tokens(
        ctx.accounts.custody_usdc_token_account.to_account_info(),
        ctx.accounts.dao_receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        usdc_amount_out,
    )?;

    // Update custody stats
    custody.assets.owned -= usdc_amount_out;

    // Completed
    genesis_lock.has_completed_otc_out = 1;

    Ok(())
}
