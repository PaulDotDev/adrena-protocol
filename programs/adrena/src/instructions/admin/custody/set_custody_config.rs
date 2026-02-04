use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            custody::{BorrowRateParams, Custody, Fees, PricingParams},
            pool::{Pool, TokenRatios, MAX_CUSTODIES},
        },
        utils::limited_string::LimitedString,
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetCustodyConfig<'info> {
    /// #1
    #[account()]
    pub admin: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"pool",
                 pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #3
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump
    )]
    pub custody: AccountLoader<'info, Custody>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SetCustodyConfigParams {
    pub is_stable: bool,
    pub oracle: LimitedString,
    pub trade_oracle: LimitedString,
    pub pricing: PricingParams,
    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,
    pub ratios: [TokenRatios; MAX_CUSTODIES],
}

pub fn set_custody_config<'info>(
    ctx: Context<'_, '_, '_, 'info, SetCustodyConfig<'info>>,
    params: &SetCustodyConfigParams,
) -> Result<u8> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    // Preliminary checks
    {
        if params.ratios.len() != pool.ratios.len() {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Stable custody cannot have trade custody as it's not tradable
        if params.is_stable {
            require_eq!(
                params.oracle.to_string(),
                params.trade_oracle.to_string(),
                AdrenaError::InvalidCustodyConfig
            );
        }
    }

    pool.ratios = params.ratios;

    require!(pool.validate(), AdrenaError::InvalidPoolConfig);

    let mut custody = ctx.accounts.custody.load_mut()?;

    {
        custody.is_stable = params.is_stable as u8;

        custody.oracle = params.oracle;
        custody.trade_oracle = params.trade_oracle;
        custody.pricing = params.pricing;
        custody.fees = params.fees;
        custody.borrow_rate = params.borrow_rate;
    }

    msg!("Custody config updated with params: {:?}", params);

    if !custody.validate() {
        err!(AdrenaError::InvalidCustodyConfig)
    } else {
        Ok(0)
    }
}
