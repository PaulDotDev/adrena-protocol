use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            custody::Custody,
            pool::{Pool, TokenRatios, MAX_CUSTODIES},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct RemoveCustody<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        mut,
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
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
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump,
        close = transfer_authority
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #7
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.token_account_bump,
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RemoveCustodyParams {
    pub ratios: [TokenRatios; MAX_CUSTODIES],
}

pub fn remove_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, RemoveCustody<'info>>,
    params: &RemoveCustodyParams,
) -> Result<u8> {
    // Preliminary checks
    {
        require!(
            ctx.accounts.custody_token_account.amount == 0,
            AdrenaError::InvalidCustodyState
        );
    }

    let mut pool = ctx.accounts.pool.load_mut()?;
    pool.remove_custody(&ctx.accounts.custody.key())?;

    pool.ratios = params.ratios;

    require!(pool.validate(), AdrenaError::InvalidPoolConfig);

    Cortex::close_token_account(
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        &[&[
            b"transfer_authority",
            &[ctx.accounts.cortex.load()?.transfer_authority_bump],
        ]],
    )?;

    Ok(0)
}
