use {
    crate::{
        error::AdrenaError,
        math,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, custody::Custody,
            oracle::Oracle, pool::Pool, staking::Staking,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    num::Zero,
};

#[derive(Accounts)]
pub struct DistributeFees<'info> {
    /// #1
    /// Anyone can call this instruction
    #[account(mut)]
    pub caller: Signer<'info>,

    /// #2
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #3
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
        has_one = fee_redistribution_mint,
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        mut,
        seeds = [b"pool",
            pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState,
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #5
    #[account(
        seeds = [b"staking", lm_staking.load()?.staked_token_mint.as_ref()],
        bump = lm_staking.load()?.bump,
        // Done in prerequisite checks to lower stack usage by anchor
        // constraint = lm_staking.load()?.is_initialized() @AdrenaError::InvalidStakingState,
        // constraint = lm_staking.load()?.staked_token_mint == cortex.load()?.lm_token_mint
    )]
    pub lm_staking: AccountLoader<'info, Staking>,

    /// #6
    #[account(
        mut,
        seeds = [b"staking", lp_token_mint.key().as_ref()],
        bump = lp_staking.load()?.bump,
        // Done in prerequisite checks to lower stack usage by anchor
        // constraint = lp_staking.load()?.is_initialized() @AdrenaError::InvalidStakingState,
        // constraint = lp_staking.load()?.staked_token_mint == lp_token_mint.key(),
    )]
    pub lp_staking: AccountLoader<'info, Staking>,

    /// #7
    #[account(
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #8
    #[account(
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #9
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #10
    #[account(
        mut,
        token::mint = cortex.load()?.fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", lm_staking.key().as_ref()],
        bump = lm_staking.load()?.reward_token_vault_bump
    )]
    pub lm_staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #11
    #[account(
        mut,
        token::mint = cortex.load()?.fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", lp_staking.key().as_ref()],
        bump = lp_staking.load()?.reward_token_vault_bump
    )]
    pub lp_staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #12
    #[account(
        init_if_needed,
        payer = caller,
        token::authority = transfer_authority,
        token::mint = fee_redistribution_mint,
        seeds = [b"referrer_reward_token_vault", cortex.load()?.fee_redistribution_mint.as_ref()],
        bump
    )]
    pub referrer_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #13
    #[account(
        mut,
        seeds = [b"custody",
                pool.key().as_ref(),
                staking_reward_token_custody.load()?.mint.as_ref()],
        bump = staking_reward_token_custody.load()?.bump,
        constraint = staking_reward_token_custody.load()?.mint == cortex.load()?.fee_redistribution_mint,
    )]
    pub staking_reward_token_custody: AccountLoader<'info, Custody>,

    /// #14
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #15
    #[account(
        mut,
        seeds = [b"custody_token_account",
                pool.key().as_ref(),
                staking_reward_token_custody.load()?.mint.as_ref()],
        bump = staking_reward_token_custody.load()?.token_account_bump,
    )]
    pub staking_reward_token_custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #16
    #[account(
        mut,
        constraint = protocol_fee_recipient.mint == cortex.load()?.fee_redistribution_mint,
    )]
    pub protocol_fee_recipient: Box<Account<'info, TokenAccount>>,

    /// #17
    pub token_program: Program<'info, Token>,

    /// #18
    pub system_program: Program<'info, System>,

    /// #19
    pub adrena_program: Program<'info, Adrena>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DistributeFeesParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

// To simplify the system, it is assumed that the fee_redistribution_mint is a stable coin and that 1 usd = 1 token
// Doing this so the referrer fee calculated in USD can be fully claimed in tokens
// as stablecoin could be send to the referrer vault as 0.99991:1 but claimed at 1:1 and create a shortage issue
pub fn distribute_fees(ctx: Context<DistributeFees>, params: &DistributeFeesParams) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    let cortex = ctx.accounts.cortex.load()?;
    let current_time = cortex.get_time()?;

    let total_fee_debt_usd = pool.fees_debt_usd + pool.referrers_fee_debt_usd;

    if total_fee_debt_usd == 0 {
        msg!("No fees to distribute");
        return Ok(());
    }

    let mut staking_reward_custody = ctx.accounts.staking_reward_token_custody.load_mut()?;

    let mut oracle = ctx.accounts.oracle.load_mut()?;

    // Preliminary checks
    {
        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }

        require!(
            ctx.accounts.lm_staking.load()?.is_initialized(),
            AdrenaError::InvalidStakingState
        );

        require!(
            ctx.accounts.lp_staking.load()?.is_initialized(),
            AdrenaError::InvalidStakingState
        );

        require!(
            ctx.accounts.lp_staking.load()?.staked_token_mint == ctx.accounts.lp_token_mint.key(),
            AdrenaError::InvalidAccountData
        );

        require!(
            ctx.accounts.lm_staking.load()?.staked_token_mint == ctx.accounts.lm_token_mint.key(),
            AdrenaError::InvalidAccountData
        );

        // Check that the price of the redistribution mint which is a stable, is not lower than 0.99
        let fixe_percent_gate = 99 * 10u64.pow((Cortex::PRICE_DECIMALS - 2).into());

        let staking_reward_token_price =
            oracle.get_oracle_price(staking_reward_custody.oracle, current_time)?;

        require!(
            staking_reward_token_price.low().price > fixe_percent_gate,
            AdrenaError::InvalidOraclePrice
        );
    }

    if total_fee_debt_usd != 0 {
        // Calculate fees repartition
        let (
            lm_stakers_fee_token_amount,
            protocol_fee_token_amount,
            referrers_fee_debt_token_amount,
        ) = {
            let lm_stakers_fee_usd =
                cortex.get_lm_stakers_fee_from_fee_amount_without_lp_fee(total_fee_debt_usd)?;

            // The referrer fee is taken out of the protocol fee
            let mut protocol_fee_usd: u64 = cortex
                .get_protocol_fee_from_fee_amount_without_lp_fee(total_fee_debt_usd)?
                - pool.referrers_fee_debt_usd;

            // Due to calculation precision, precision loss can occur
            // Send all remaining to protocol fee
            protocol_fee_usd += total_fee_debt_usd
                - (lm_stakers_fee_usd + protocol_fee_usd + pool.referrers_fee_debt_usd);

            // Make sure the math are math-ing
            assert!(
                total_fee_debt_usd
                    == (lm_stakers_fee_usd + protocol_fee_usd + pool.referrers_fee_debt_usd)
            );

            // Here assume redistribution mint and usd are 1:1
            (
                lm_stakers_fee_usd,
                protocol_fee_usd,
                pool.referrers_fee_debt_usd,
            )
        };

        let total_token_amount_to_send = lm_stakers_fee_token_amount
            + protocol_fee_token_amount
            + referrers_fee_debt_token_amount;

        msg!(
            "total_token_amount_to_send: {:?}",
            total_token_amount_to_send
        );

        // Verify there are enough tokens to pay the fees
        if staking_reward_custody.assets.locked + total_token_amount_to_send
            > staking_reward_custody.assets.owned
        {
            return Err(AdrenaError::InsufficientCollateral.into());
        }

        // Distribute fees
        {
            // Transfer to LM stakers
            cortex.transfer_tokens(
                ctx.accounts
                    .staking_reward_token_custody_token_account
                    .to_account_info(),
                ctx.accounts.lm_staking_reward_token_vault.to_account_info(),
                ctx.accounts.transfer_authority.clone(),
                ctx.accounts.token_program.to_account_info(),
                lm_stakers_fee_token_amount,
            )?;

            // For liquid LP, what is in the pool, stays in the pool, already done before fees distribution
            // ...

            // Transfer to referrers
            cortex.transfer_tokens(
                ctx.accounts
                    .staking_reward_token_custody_token_account
                    .to_account_info(),
                ctx.accounts.referrer_reward_token_vault.to_account_info(),
                ctx.accounts.transfer_authority.clone(),
                ctx.accounts.token_program.to_account_info(),
                referrers_fee_debt_token_amount,
            )?;

            // Transfer to protocol fee recipient
            cortex.transfer_tokens(
                ctx.accounts
                    .staking_reward_token_custody_token_account
                    .to_account_info(),
                ctx.accounts.protocol_fee_recipient.to_account_info(),
                ctx.accounts.transfer_authority.clone(),
                ctx.accounts.token_program.to_account_info(),
                protocol_fee_token_amount,
            )?;
        }

        // Update custody accounting
        {
            staking_reward_custody.assets.owned -= total_token_amount_to_send;
        }

        msg!("Previous value: {}", pool.aum_usd.to_u128());

        // Reset pool fees debt (as it has been paid)
        {
            pool.fees_debt_usd = 0;
            pool.referrers_fee_debt_usd = 0;
        }

        drop(staking_reward_custody);
    }

    // Update AUM and LP token price
    {
        let aum_usd =
            pool.get_assets_under_management_usd(&oracle, ctx.remaining_accounts, current_time)?;

        pool.aum_usd = U128Split::new(aum_usd);

        let lp_supply = ctx.accounts.lp_token_mint.supply;

        pool.lp_token_price_usd = if lp_supply.is_zero() {
            0
        } else {
            math::checked_decimal_div(
                math::checked_as_u64(aum_usd)?,
                -(Cortex::USD_DECIMALS as i32),
                lp_supply,
                -(Cortex::LP_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?
        };

        pool.last_aum_and_lp_token_price_usd_update = current_time;
    }

    if total_fee_debt_usd != 0 {
        staking_reward_custody = ctx.accounts.staking_reward_token_custody.load_mut()?;

        // Refresh borrow rate
        staking_reward_custody.update_borrow_rate(current_time)?;
    }

    Ok(())
}
