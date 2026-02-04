use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{Pool, StableCustodyInfo},
            position::Side,
        },
        utils::u128_split::U128Split,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Mint,
    num::Zero,
};

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct CustodyInfoSnapshot {
    pub assets_value_usd: u64,
    pub owned: u64,
    pub locked: u64,
    pub price: u64,
    pub price_confidence: u64,
    pub trade_price: u64,
    pub trade_price_confidence: u64,
    pub short_pnl: i64,
    pub long_pnl: i64,
    pub open_interest_long_usd: u64,
    pub open_interest_short_usd: u64,
    pub cumulative_profit_usd: u64,
    pub cumulative_loss_usd: u64,
    pub cumulative_swap_fee_usd: u64,
    pub cumulative_liquidity_fee_usd: u64,
    pub cumulative_close_position_fee_usd: u64,
    pub cumulative_liquidation_fee_usd: u64,
    pub cumulative_borrow_fee_usd: u64,
    pub cumulative_trading_volume_usd: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PoolInfoSnapshot {
    pub current_time: u64,
    pub aum_usd: u64,
    pub lp_token_price: u64,
    pub custodies_info_snapshot: Vec<CustodyInfoSnapshot>,
    pub lp_circulating_supply: u64,
    pub cumulative_referrer_fee_usd: u64,
}

#[derive(Accounts)]
pub struct GetPoolInfoSnapshot<'info> {
    /// #1
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

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
        seeds = [b"lp_token_mint",
            pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #4
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.load()?.bump
    )]
    pub oracle: AccountLoader<'info, Oracle>,
    //
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GetPoolInfoSnapshotParams {
    // Do not do that, except if you know the onchain price is fresh (i.e you did just update the price in a prior instruction or this is CPI)
    pub oracle_prices: Option<ChaosLabsBatchPrices>,
}

pub fn get_pool_info_snapshot(
    ctx: Context<GetPoolInfoSnapshot>,
    params: &GetPoolInfoSnapshotParams,
) -> Result<PoolInfoSnapshot> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut custodies_info_snapshot = Vec::new();

    let custodies = pool.get_custodies();
    let accounts = ctx.remaining_accounts;

    let current_time = ctx.accounts.cortex.load()?.get_time()?;
    let mut oracle = ctx.accounts.oracle.load_mut()?;

    // Preliminary checks
    {
        // Pick freshest price after we checked the given prices are correct
        if let Some(ref prices) = params.oracle_prices {
            oracle.verify_and_update_prices(prices, current_time)?;
        }
    }

    // This code is very similar with get_assets_under_management
    let aum_usd = {
        let stable_custodies_info = {
            let mut stable_custodies_info: Vec<StableCustodyInfo> = vec![];

            for (idx, &custody_pubkey) in custodies.iter().enumerate() {
                require_keys_eq!(accounts[idx].key(), custody_pubkey);

                let mut data: &[u8] = &accounts[idx]
                    .data
                    .try_borrow_mut()
                    .map_err(|_| AdrenaError::InvalidCustodyAccount)?;

                let custody = Custody::try_deserialize(&mut data)
                    .map_err(|_| AdrenaError::InvalidCustodyAccount)?;

                if !custody.is_stable() {
                    continue;
                }

                let stable_token_price = oracle.get_oracle_price(custody.oracle, current_time)?;

                stable_custodies_info.push(StableCustodyInfo {
                    custody: accounts[idx].key(),
                    token_price: stable_token_price.high(),
                    decimals: custody.decimals,
                });
            }

            stable_custodies_info
        };

        let mut pool_value_sub_usd: u128 = 0;
        let mut pool_value_add_usd: u128 = 0;

        for (idx, &custody_pubkey) in custodies.iter().enumerate() {
            require_keys_eq!(accounts[idx].key(), custody_pubkey);

            let mut data: &[u8] = &accounts[idx]
                .data
                .try_borrow_mut()
                .map_err(|_| AdrenaError::InvalidCustodyAccount)?;
            let custody = Custody::try_deserialize(&mut data)
                .map_err(|_| AdrenaError::InvalidCustodyAccount)?;

            let token_price = oracle.get_oracle_price(custody.oracle, current_time)?;
            let token_trade_price = oracle.get_oracle_price(custody.trade_oracle, current_time)?;

            let token_amount_usd = token_price
                .low()
                .get_asset_amount_usd(custody.assets.owned, custody.decimals)?;

            pool_value_add_usd += token_amount_usd as u128;

            let (short_pnl, long_pnl) = {
                if custody.is_stable() {
                    // compute accumulated interest
                    let collective_position = custody.get_collective_position(Side::Long)?;

                    let unrealized_interest_usd =
                        custody.get_interest_amount_usd(&collective_position, current_time)?;

                    pool_value_add_usd += unrealized_interest_usd as u128;

                    let collective_position = custody.get_collective_position(Side::Short)?;
                    let unrealized_interest_usd =
                        custody.get_interest_amount_usd(&collective_position, current_time)?;

                    pool_value_add_usd += unrealized_interest_usd as u128;

                    (0_i64, 0_i64)
                } else {
                    // compute long aggregate unrealized pnl
                    let long_pnl = pool.get_pnl_usd(
                        &custody.get_collective_position(Side::Long)?,
                        &token_trade_price,
                        &token_price.high(),
                        &custody,
                        current_time,
                        false,
                    )?;

                    let short_collective_position = {
                        let mut short_collective_position =
                            custody.get_collective_position(Side::Short)?;

                        // When calculating PnL, we need to know the amount of token locked for payout
                        // to know the max loss or max profit
                        //
                        // A way to do it is to store in locked_amount an amount of custody token equivalent to the total
                        // stable collateral value locked in the protocol
                        //
                        // Not 100% clean, but work with how get_pnl_usd works today

                        let mut total_stable_locked_amount_usd: u64 = 0;

                        for stable_locked_amount in
                            custody.short_positions.stable_locked_amount.iter()
                        {
                            if stable_locked_amount.locked_amount == 0 {
                                continue;
                            }

                            let stable_custody_info = stable_custodies_info
                                .iter()
                                .find(|info| info.custody.eq(&stable_locked_amount.custody))
                                .ok_or(AdrenaError::CustodyNotFound)?;

                            let stable_locked_amount_usd = stable_custody_info
                                .token_price
                                .high()
                                .get_asset_amount_usd(
                                stable_locked_amount.locked_amount,
                                stable_custody_info.decimals,
                            )?;

                            total_stable_locked_amount_usd += stable_locked_amount_usd;
                        }

                        short_collective_position.locked_amount = token_price
                            .low()
                            .get_token_amount(total_stable_locked_amount_usd, custody.decimals)?;

                        short_collective_position
                    };

                    let short_pnl = pool.get_pnl_usd(
                        &short_collective_position,
                        &token_trade_price,
                        &token_price,
                        &custody,
                        current_time,
                        false,
                    )?;

                    // Calculate collateral PnL for the pool (the pool is exposed long 1x on collateral provided by traders for Long positions)
                    {
                        let current_long_positions_collateral_usd = token_price
                            .low()
                            .get_asset_amount_usd(custody.assets.collateral, custody.decimals)?;

                        let previous_long_positions_collateral_usd =
                            custody.long_positions.collateral_usd;

                        match previous_long_positions_collateral_usd
                            .cmp(&current_long_positions_collateral_usd)
                        {
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Less => {
                                // The pool is making money on collateral
                                let profit_usd = current_long_positions_collateral_usd
                                    - previous_long_positions_collateral_usd;

                                pool_value_add_usd += profit_usd as u128;
                            }
                            std::cmp::Ordering::Greater => {
                                // The pool is losing money on collateral
                                let loss_usd = previous_long_positions_collateral_usd
                                    - current_long_positions_collateral_usd;

                                pool_value_sub_usd += loss_usd as u128;
                            }
                        }
                    }

                    // Adjust pool amount by collective profit/loss

                    // Unrealized position losses are considered part of the pool
                    pool_value_add_usd += long_pnl.loss_usd as u128;
                    pool_value_add_usd += short_pnl.loss_usd as u128;

                    // Unrealized position profits are considered out of the pool
                    pool_value_sub_usd += long_pnl.profit_usd as u128;
                    pool_value_sub_usd += short_pnl.profit_usd as u128;

                    (
                        math::checked_as_i64(short_pnl.profit_usd)?
                            - math::checked_as_i64(short_pnl.loss_usd)?,
                        math::checked_as_i64(long_pnl.profit_usd)?
                            - math::checked_as_i64(long_pnl.loss_usd)?,
                    )
                }
            };

            // Consolidate info about the custody
            custodies_info_snapshot.push(CustodyInfoSnapshot {
                assets_value_usd: token_amount_usd,
                owned: custody.assets.owned,
                locked: custody.assets.locked,
                price: token_price.price,
                price_confidence: token_price.confidence,
                trade_price: token_trade_price.price,
                trade_price_confidence: token_trade_price.confidence,
                // Actual Data
                short_pnl,
                long_pnl,
                open_interest_long_usd: custody.long_positions.size_usd,
                open_interest_short_usd: custody.short_positions.size_usd,
                // Cumulative Data
                cumulative_profit_usd: custody.trade_stats.profit_usd,
                cumulative_loss_usd: custody.trade_stats.loss_usd,

                cumulative_swap_fee_usd: custody.collected_fees.swap_usd,
                cumulative_liquidity_fee_usd: custody.collected_fees.add_liquidity_usd
                    + custody.collected_fees.remove_liquidity_usd,
                cumulative_close_position_fee_usd: custody.collected_fees.close_position_usd,
                cumulative_liquidation_fee_usd: custody.collected_fees.liquidation_usd,
                cumulative_borrow_fee_usd: custody.collected_fees.borrow_usd,
                cumulative_trading_volume_usd: custody.volume_stats.close_position_usd
                    + custody.volume_stats.liquidation_usd
                    + custody.volume_stats.open_position_usd,
            });
        }

        math::checked_as_u64(pool_value_add_usd.saturating_sub(pool_value_sub_usd))?
    };

    let lp_supply = ctx.accounts.lp_token_mint.supply;

    let lp_token_price_usd = if lp_supply.is_zero() {
        0
    } else {
        math::checked_decimal_div(
            aum_usd,
            -(Cortex::USD_DECIMALS as i32),
            lp_supply,
            -(Cortex::LP_DECIMALS as i32),
            -(Cortex::PRICE_DECIMALS as i32),
        )?
    };

    // Update AUM and LP token price
    {
        pool.aum_usd = U128Split::new(aum_usd as u128);
        pool.last_aum_and_lp_token_price_usd_update = current_time;
        pool.lp_token_price_usd = lp_token_price_usd;
    }

    Ok(PoolInfoSnapshot {
        current_time: math::checked_as_u64(current_time)?,
        aum_usd, // Contains PnL
        custodies_info_snapshot,
        lp_token_price: lp_token_price_usd,
        lp_circulating_supply: ctx.accounts.lp_token_mint.supply,
        cumulative_referrer_fee_usd: pool.cumulative_referrer_fee_usd,
    })
}
