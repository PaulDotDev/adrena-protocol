use {
    crate::{
        error::AdrenaError,
        math,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{Pool, PoolLiquidityState},
            staking::Staking,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    num::Zero,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    /// #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = funding_account.mint == custody.load()?.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #3 Front end will target the owner account, but not limited to
    #[account(
        mut,
        constraint = lp_token_account.mint == lp_token_mint.key(),
    )]
    pub lp_token_account: Box<Account<'info, TokenAccount>>,

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
        seeds = [b"staking", lp_token_mint.key().as_ref()],
        bump = lp_staking.load()?.bump,
        constraint = lp_staking.load()?.is_initialized() @AdrenaError::InvalidStakingState,
    )]
    pub lp_staking: AccountLoader<'info, Staking>,

    /// #6
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #7
    #[account(
        mut,
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
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.bump
    )]
    pub custody: AccountLoader<'info, Custody>,

    /// #9
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,

    /// #10
    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.load()?.mint.as_ref()],
        bump = custody.load()?.token_account_bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    /// #11
    #[account(
        mut,
        seeds = [b"lp_token_mint",
        pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #12
    pub token_program: Program<'info, Token>,

    /// #13
    pub adrena_program: Program<'info, Adrena>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AddLiquidityParams {
    pub amount_in: u64,
    pub min_lp_amount_out: u64,
    // If not used, then will use the price that is onchain
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn add_liquidity(ctx: Context<AddLiquidity>, params: &AddLiquidityParams) -> Result<()> {
    let cortex = ctx.accounts.cortex.load_mut()?;
    let mut custody = ctx.accounts.custody.load_mut()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;

    let current_time = cortex.get_time()?;

    // Preliminary checks
    {
        require!(
            ctx.accounts.lp_staking.load()?.staked_token_mint == ctx.accounts.lp_token_mint.key(),
            AdrenaError::InvalidAccountData
        );

        require!(
            pool.get_liquidity_state().eq(&PoolLiquidityState::Active),
            AdrenaError::InstructionNotAllowed
        );

        if params.amount_in == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    let token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

    // In order to protect the protocol from volatility, we apply a confidence
    let token_price_low = token_price.low();
    msg!(
        "Current price: {} (low {})",
        token_price.price,
        token_price_low.price
    );

    msg!("Amount in: {}", params.amount_in);

    {
        // Must drop to let get assets under management load remaining accounts about the same custody
        drop(custody);

        // Refresh pool.aum_usm to adapt to token price change
        pool.aum_usd = U128Split::new(pool.get_assets_under_management_usd(
            &oracle,
            ctx.remaining_accounts,
            current_time,
        )?);

        custody = ctx.accounts.custody.load_mut()?;
    }

    msg!("Pool AUM (usd): {}", pool.aum_usd.to_u128());

    let fee_amount = pool.get_add_liquidity_fee(params.amount_in, &custody)?;

    msg!("Collected fee: {}", fee_amount);

    let deposit_amount = params.amount_in;

    cortex.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount_in,
    )?;

    let pool_amount_usd = {
        // Must drop to let get assets under management load remaining accounts about the same custody
        drop(custody);

        // Refresh pool.aum_usm to adapt to token price change
        let pool_amount_usd =
            pool.get_assets_under_management_usd(&oracle, ctx.remaining_accounts, current_time)?;

        custody = ctx.accounts.custody.load_mut()?;

        pool_amount_usd
    };

    // compute amount of lp tokens to mint
    let no_fee_amount = params.amount_in - fee_amount;

    msg!("No fee amount: {}", no_fee_amount);

    require_gte!(no_fee_amount, 1u64, AdrenaError::InsufficientAmountReturned);

    // TODO: I think this aum refresh is not needed - because it doesn't adapt to new tokens in the custody, as only "owned" tokens are considered
    msg!("Pool AUM (usd): {}", pool_amount_usd);

    let lp_fee = cortex.get_lp_fee(fee_amount)?;
    let lp_fee_usd = token_price_low.get_asset_amount_usd(lp_fee, custody.decimals)?;

    let token_amount_usd = token_price_low.get_asset_amount_usd(no_fee_amount, custody.decimals)?;

    let lp_amount = if pool_amount_usd == 0 {
        token_amount_usd
    } else {
        math::checked_as_u64(
            (token_amount_usd as u128 * ctx.accounts.lp_token_mint.supply as u128)
                / (pool_amount_usd + lp_fee_usd as u128),
        )?
    };

    msg!("LP token mint: {}", ctx.accounts.lp_token_mint.supply);
    msg!("LP tokens to mint: {}", lp_amount);

    require!(
        lp_amount >= params.min_lp_amount_out,
        AdrenaError::MaxPriceSlippage
    );

    // Mint lp tokens
    {
        cortex.mint_tokens(
            ctx.accounts.lp_token_mint.to_account_info(),
            ctx.accounts.lp_token_account.to_account_info(),
            ctx.accounts.transfer_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            lp_amount,
        )?;

        ctx.accounts.lp_token_mint.reload()?;
    }

    // Update custody stats
    {
        let fee_usd = token_price.get_asset_amount_usd(fee_amount, custody.decimals)?;

        custody.collected_fees.add_liquidity_usd = custody
            .collected_fees
            .add_liquidity_usd
            .wrapping_add(fee_usd);

        custody.volume_stats.add_liquidity_usd =
            custody.volume_stats.add_liquidity_usd.wrapping_add(
                token_price_low.get_asset_amount_usd(params.amount_in, custody.decimals)?,
            );

        // Contains fees to be distributed later
        custody.assets.owned += deposit_amount;
    }

    // Update fee debt accounting
    {
        // NOTE: Do not count LP fee as part of the fee debt as the fee stay in the pool
        let fee_amount_less_lp_fee = fee_amount - lp_fee;

        let fee_usd = token_price.get_asset_amount_usd(fee_amount_less_lp_fee, custody.decimals)?;

        pool.fees_debt_usd += fee_usd;
    }

    // Update pool AUM and borrow rate (because we added tokens as "owned" in the custody)
    {
        // Must drop to let get assets under management load remaining accounts about the same custody
        drop(custody);

        let aum_usd =
            pool.get_assets_under_management_usd(&oracle, ctx.remaining_accounts, current_time)?;

        pool.aum_usd = U128Split::new(aum_usd);

        msg!("Pool AUM (usd): {}", pool.aum_usd.to_u128());

        require!(
            pool.aum_usd.to_u128() < pool.aum_soft_cap_usd as u128,
            AdrenaError::PoolAumSoftCapUsdReached
        );

        // Update LP token price
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

        custody = ctx.accounts.custody.load_mut()?;

        custody.update_borrow_rate(current_time)?;
    }

    Ok(())
}
