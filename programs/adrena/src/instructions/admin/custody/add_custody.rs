use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            custody::{BorrowRateParams, Custody, Fees, PricingParams},
            oracle::Oracle,
            pool::{Pool, TokenRatios, MAX_CUSTODIES},
        },
        utils::limited_string::LimitedString,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct AddCustody<'info> {
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
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        init,
        payer = payer,
        space = Custody::LEN,
        seeds = [b"custody",
                pool.key().as_ref(),
                custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #7
    #[account(
        init,
        payer = payer,
        token::mint = custody_token_mint,
        token::authority = transfer_authority,
        seeds = [b"custody_token_account",
                pool.key().as_ref(),
                custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #8
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #9
    pub custody_token_mint: Box<Account<'info, Mint>>,

    /// #10
    pub system_program: Program<'info, System>,

    /// #11
    pub token_program: Program<'info, Token>,

    /// #12
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddCustodyParams {
    pub is_stable: bool,
    pub pricing: PricingParams,
    pub allow_swap: bool,
    pub allow_trade: bool,
    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,
    pub ratios: [TokenRatios; MAX_CUSTODIES],
    pub oracle: LimitedString,
    pub trade_oracle: LimitedString,
}

pub fn add_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, AddCustody<'info>>,
    params: &AddCustodyParams,
) -> Result<u8> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    if pool.get_token_id(&ctx.accounts.custody.key()).is_ok() {
        return Err(ProgramError::AccountAlreadyInitialized.into());
    }

    // Stable custody cannot have trade custody as it's not tradable
    if params.is_stable {
        require_eq!(
            params.oracle.to_string(),
            params.trade_oracle.to_string(),
            AdrenaError::InvalidCustodyConfig
        );
    }

    // Update pool data
    {
        // Add the custody to the pool
        pool.add_custody(ctx.accounts.custody.key())?;

        pool.ratios = params.ratios;

        if params.is_stable {
            pool.nb_stable_custody += 1;
        }
    }

    if !pool.validate() {
        return err!(AdrenaError::InvalidPoolConfig);
    }

    let mut custody = ctx.accounts.custody.load_init()?;

    // Set custody values
    {
        custody.pool = ctx.accounts.pool.key();
        custody.mint = ctx.accounts.custody_token_mint.key();
        custody.token_account = ctx.accounts.custody_token_account.key();
        custody.decimals = ctx.accounts.custody_token_mint.decimals;
        custody.is_stable = params.is_stable as u8;
        custody.oracle = params.oracle;
        custody.trade_oracle = params.trade_oracle;
        custody.pricing = params.pricing;
        custody.allow_swap = params.allow_swap as u8;
        custody.allow_trade = params.allow_trade as u8;
        custody.fees = params.fees;
        custody.borrow_rate = params.borrow_rate;
        custody.borrow_rate_state.current_rate = 0;
        custody.borrow_rate_state.last_update = ctx.accounts.cortex.load()?.get_time()?;
        custody.bump = ctx.bumps.custody;
        custody.token_account_bump = ctx.bumps.custody_token_account;
    }

    require!(custody.validate(), AdrenaError::InvalidCustodyConfig);

    Ok(0)
}
