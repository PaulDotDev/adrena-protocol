use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            oracle::{Oracle, OraclePrice},
        },
        utils::limited_string::LimitedString,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
    std::collections::HashSet,
};

#[derive(Accounts)]
pub struct InitOracle<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        init,
        payer = payer,
        space = Oracle::LEN,
        seeds = [b"oracle"],
        bump,
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #5
    pub system_program: Program<'info, System>,

    /// #6
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OraclePricesSetup {
    pub name: LimitedString,
    pub chaos_labs_feed_id: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitOracleParams {
    pub oracle_prices: Vec<OraclePricesSetup>,
}

pub fn init_oracle(ctx: Context<InitOracle>, params: &InitOracleParams) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;

    // Preliminary checks
    {
        // If too many prices
        if params.oracle_prices.len() > crate::state::oracle::MAX_ORACLE_PRICES_COUNT {
            return Err(ProgramError::InvalidArgument.into());
        }

        let mut feed_ids = HashSet::new();
        let mut names = HashSet::new();

        for price in &params.oracle_prices {
            // check for duplicate chaos_labs_feed_id
            if !feed_ids.insert(price.chaos_labs_feed_id) {
                msg!(
                    "Duplicated chaos_labs_feed_id: {}",
                    price.chaos_labs_feed_id
                );

                return Err(ProgramError::InvalidArgument.into());
            }

            // check for duplicate name
            if !names.insert(price.name.to_string()) {
                msg!("Duplicated name: {}", price.name);

                return Err(ProgramError::InvalidArgument.into());
            }
        }
    }

    oracle.bump = ctx.bumps.oracle;
    oracle.updated_at = ctx.accounts.cortex.load()?.get_time()?;

    for (i, op) in params.oracle_prices.iter().enumerate() {
        oracle.prices[i] = OraclePrice {
            price: 0,
            confidence: 0,
            timestamp: 0, // Price will be stale, so not usable, requires at least one update first
            exponent: -(Cortex::PRICE_DECIMALS as i32),
            chaos_labs_feed_id: op.chaos_labs_feed_id,
            name: op.name,
            _padding: Default::default(),
        };
    }

    Ok(())
}
