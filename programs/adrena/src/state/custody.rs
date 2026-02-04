use {
    crate::{
        error::AdrenaError,
        math,
        state::{
            cortex::Cortex,
            oracle::OraclePrice,
            position::{Position, Side},
        },
        utils::{limited_string::LimitedString, u128_split::U128Split},
    },
    anchor_lang::prelude::*,
    bytemuck::{Pod, Zeroable},
};

pub const MAX_STABLE_CUSTODY: usize = 2;

// in BPS
pub const MIN_INITIAL_LEVERAGE: u32 = 11_000;

// Fees have implied BPS_DECIMALS decimals
#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct Fees {
    // Base fees
    pub swap_in: u16,
    pub swap_out: u16,
    pub stable_swap_in: u16,
    pub stable_swap_out: u16,
    pub add_liquidity: u16,
    pub remove_liquidity: u16,
    pub close_position: u16,
    pub liquidation: u16,
    pub fee_max: u16,
    pub _padding: [u8; 6],
    pub _padding2: u64, // force u64 alignment
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct FeesStats {
    pub swap_usd: u64,
    pub add_liquidity_usd: u64,
    pub remove_liquidity_usd: u64,
    pub close_position_usd: u64,
    pub liquidation_usd: u64,
    pub borrow_usd: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct VolumeStats {
    pub swap_usd: u64,
    pub add_liquidity_usd: u64,
    pub remove_liquidity_usd: u64,
    pub open_position_usd: u64,
    pub close_position_usd: u64,
    pub liquidation_usd: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct TradeStats {
    pub profit_usd: u64,
    pub loss_usd: u64,
    pub oi_long_usd: u64,
    pub oi_short_usd: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct Assets {
    // Collateral debt
    pub collateral: u64,
    // Owned = total_assets - collateral + collected_fees - protocol_fees
    pub owned: u64,
    // Locked funds for pnl payoff
    pub locked: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct PricingParams {
    // Pricing params have implied BPS_DECIMALS decimals (except ended with _usd)
    pub max_initial_leverage: u32,
    pub max_leverage: u32,
    // One position size can't exceed this amount
    pub max_position_locked_usd: u64,
    // Limit the total size of short positions for the custody
    pub max_cumulative_short_position_size_usd: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct BorrowRateParams {
    // Borrow rate params have implied RATE_DECIMALS decimals
    pub max_hourly_borrow_interest_rate: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct BorrowRateState {
    // Borrow rates have implied RATE_DECIMALS decimals
    pub current_rate: u64,
    pub last_update: i64,
    pub cumulative_interest: U128Split,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct PositionsAccounting {
    pub open_positions: u64,
    pub size_usd: u64,
    pub borrow_size_usd: u64,
    pub locked_amount: u64,
    pub weighted_price: U128Split,
    pub total_quantity: U128Split,
    // interests due (unrealized)
    pub cumulative_interest_usd: u64,
    pub collateral_usd: u64, // Stat only used for long positions
    pub cumulative_interest_snapshot: U128Split,
    // This exit fee stats is used to calculate the PnL of all opened positions, it is not reflecting the actual exit fee of the position (that can sometimes be lesser)
    // so that the AUM take into account an approximation of the PnL of all opened positions
    pub exit_fee_usd: u64,
    //
    // Store the stable custody locked amount related to this custody
    //
    // Example:
    // When Shorting 1 ETH, 1500 USDC get locked to provide for trader maximum payoff
    // USDC custody locked amount: +1500
    // eth custody stable locked amount: +1500
    //
    // Needed to be able to calculate PnL
    pub stable_locked_amount: [StableLockedAmountStat; MAX_STABLE_CUSTODY],
}

impl PositionsAccounting {
    pub fn get_custody_stable_locked_amount(
        &mut self,
        stable_custody: &Pubkey,
    ) -> Result<&mut StableLockedAmountStat> {
        // Find the index of the stable custody within the array
        if let Some(index) =
            self.stable_locked_amount
                .iter()
                .position(|stable_locked_amount_stat| {
                    stable_locked_amount_stat.custody.eq(stable_custody)
                })
        {
            return Ok(&mut self.stable_locked_amount[index]);
        }

        // If the stable custody is not in the array, find the index of an empty spot
        if let Some(index) = self
            .stable_locked_amount
            .iter()
            .position(|stable_locked_amount_stat| stable_locked_amount_stat.locked_amount.eq(&0u64))
        {
            self.stable_locked_amount[index].custody = *stable_custody;

            return Ok(&mut self.stable_locked_amount[index]);
        }

        Err(AdrenaError::CustodyStableLockedAmountNotFound.into())
    }

    // Consider stables to be worth 1$ - does not introduce risk, even if stable depegged
    pub fn get_total_shorted_amount_usd(&self) -> u64 {
        self.stable_locked_amount
            .iter()
            .map(|e| e.locked_amount)
            .sum()
    }
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct StableLockedAmountStat {
    pub custody: Pubkey,
    pub locked_amount: u64,
    pub _padding: [u8; 8],
}

#[account(zero_copy)]
#[derive(Default, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct Custody {
    pub bump: u8,
    pub token_account_bump: u8,
    //
    // Permissions
    //
    // If false, make positions readonly/closeonly
    pub allow_trade: u8,
    pub allow_swap: u8,
    //
    pub decimals: u8,
    pub is_stable: u8,
    pub _padding: [u8; 2],
    //
    pub pool: Pubkey,
    // /!\ The position of this field matters as we use the offset to get the mint in get_custody_mint_from_account_info
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub oracle: LimitedString,
    pub trade_oracle: LimitedString,
    pub pricing: PricingParams,

    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,
    // All time stats
    pub collected_fees: FeesStats,
    pub volume_stats: VolumeStats,
    pub trade_stats: TradeStats,
    // Real time stats
    pub assets: Assets,
    pub long_positions: PositionsAccounting,
    pub short_positions: PositionsAccounting,
    pub borrow_rate_state: BorrowRateState,
}

pub fn get_custody_mint_from_account_info(account_info: &AccountInfo<'_>) -> Pubkey {
    // anchor discriminator 8 + 8 (bumps + u8s) + 32 pool pubkey
    let data_slice = &account_info.data.borrow()[48..80];

    let mut pubkey_bytes = [0u8; 32];

    pubkey_bytes.copy_from_slice(data_slice);

    Pubkey::from(pubkey_bytes)
}

impl Fees {
    pub fn validate(&self) -> bool {
        self.swap_in as u128 <= Cortex::BPS_POWER
            && self.swap_out as u128 <= Cortex::BPS_POWER
            && self.stable_swap_in as u128 <= Cortex::BPS_POWER
            && self.stable_swap_out as u128 <= Cortex::BPS_POWER
            && self.add_liquidity as u128 <= Cortex::BPS_POWER
            && self.remove_liquidity as u128 <= Cortex::BPS_POWER
            && self.close_position as u128 <= Cortex::BPS_POWER
            && self.liquidation as u128 <= Cortex::BPS_POWER
            && self.fee_max as u128 <= Cortex::BPS_POWER
    }
}

impl PricingParams {
    pub fn validate(&self) -> bool {
        (MIN_INITIAL_LEVERAGE as u128) >= Cortex::BPS_POWER
            && MIN_INITIAL_LEVERAGE <= self.max_initial_leverage
            && self.max_initial_leverage <= self.max_leverage
    }
}

impl BorrowRateParams {
    pub fn validate(&self) -> bool {
        // Between 0%+ and 100%
        self.max_hourly_borrow_interest_rate > 0
            && (self.max_hourly_borrow_interest_rate as u128) <= Cortex::RATE_POWER
    }
}

impl Custody {
    pub const LEN: usize = 8 + std::mem::size_of::<Custody>();

    pub fn validate(&self) -> bool {
        self.token_account != Pubkey::default()
            && self.mint != Pubkey::default()
            && self.oracle != LimitedString::default()
            && self.trade_oracle != LimitedString::default()
            && self.pricing.validate()
            && self.fees.validate()
            && self.borrow_rate.validate()
    }

    pub fn is_stable(&self) -> bool {
        self.is_stable == 1
    }

    pub fn allow_trade(&self) -> bool {
        self.allow_trade == 1
    }

    pub fn allow_swap(&self) -> bool {
        self.allow_swap == 1
    }

    pub fn lock_funds(&mut self, amount: u64) -> Result<()> {
        self.assets.locked += amount;

        if self.assets.owned < self.assets.locked {
            Err(ProgramError::InsufficientFunds.into())
        } else {
            Ok(())
        }
    }

    pub fn unlock_funds(&mut self, amount: u64) -> Result<()> {
        if amount > self.assets.locked {
            self.assets.locked = 0;
        } else {
            self.assets.locked -= amount;
        }

        Ok(())
    }

    // Returns the interest amount that has accrued since the last position cumulative interest snapshot update
    pub fn get_interest_amount_usd(&self, position: &Position, current_time: i64) -> Result<u64> {
        if position.borrow_size_usd == 0 {
            return Ok(0);
        }

        let cumulative_interest = self.get_cumulative_interest(current_time)?;

        let position_interest =
            if cumulative_interest > position.cumulative_interest_snapshot.to_u128() {
                cumulative_interest - position.cumulative_interest_snapshot.to_u128()
            } else {
                return Ok(0);
            };

        math::checked_as_u64(
            (position_interest * position.borrow_size_usd as u128) / Cortex::RATE_POWER,
        )
    }

    pub fn get_cumulative_interest(&self, current_time: i64) -> Result<u128> {
        if current_time > self.borrow_rate_state.last_update {
            let cumulative_interest = math::checked_ceil_div(
                (current_time - self.borrow_rate_state.last_update) as u128
                    * self.borrow_rate_state.current_rate as u128,
                3_600,
            )?;

            Ok(self.borrow_rate_state.cumulative_interest.to_u128() + cumulative_interest)
        } else {
            Ok(self.borrow_rate_state.cumulative_interest.to_u128())
        }
    }

    pub fn update_borrow_rate(&mut self, current_time: i64) -> Result<()> {
        if self.assets.owned == 0 {
            self.borrow_rate_state.current_rate = 0;
            self.borrow_rate_state.last_update =
                std::cmp::max(current_time, self.borrow_rate_state.last_update);
            return Ok(());
        }

        if current_time > self.borrow_rate_state.last_update {
            // compute interest accumulated since previous update
            self.borrow_rate_state.cumulative_interest =
                self.get_cumulative_interest(current_time)?.into();
            self.borrow_rate_state.last_update = current_time;
        }

        // the borrow rate is now a function of the current utilization, where its max value is the max_hourly_borrow_interest_rate and the min value near 0
        let hourly_rate = math::checked_ceil_div(
            (self.assets.locked as u128
                * self.borrow_rate.max_hourly_borrow_interest_rate as u128
                * Cortex::RATE_POWER)
                / self.assets.owned as u128,
            Cortex::RATE_POWER,
        )?;

        self.borrow_rate_state.current_rate = math::checked_as_u64(hourly_rate)?;

        require!(
            self.borrow_rate_state.current_rate <= self.borrow_rate.max_hourly_borrow_interest_rate,
            AdrenaError::InvalidCustodyConfig
        );

        Ok(())
    }

    pub fn get_collective_position(&self, side: Side) -> Result<Position> {
        let accounting = if side == Side::Long {
            &self.long_positions
        } else {
            &self.short_positions
        };

        if accounting.open_positions > 0 {
            Ok(Position {
                side: side.into(),
                price: if accounting.total_quantity.to_u128() > 0 {
                    math::checked_as_u64(
                        accounting.weighted_price.to_u128() / accounting.total_quantity.to_u128(),
                    )?
                } else {
                    0
                },
                size_usd: accounting.size_usd,
                borrow_size_usd: accounting.borrow_size_usd,
                unrealized_interest_usd: accounting.cumulative_interest_usd,
                cumulative_interest_snapshot: accounting.cumulative_interest_snapshot,
                locked_amount: accounting.locked_amount,
                exit_fee_usd: accounting.exit_fee_usd,
                ..Position::default()
            })
        } else {
            Ok(Position::default())
        }
    }

    pub fn update_accounting_after_open_position_short(
        &mut self,
        position: &Position,
        collateral_token_price: &OraclePrice,
        current_time: i64,
        collateral_custody: &mut Custody,
    ) -> Result<()> {
        // Update custody accounting
        let accounting = {
            let accounting = &mut self.short_positions;

            accounting.open_positions += 1;
            accounting.size_usd += position.size_usd;
            accounting.exit_fee_usd += position.exit_fee_usd;

            accounting
        };

        // To be able to calculate PnL properly, needs to store on custody accounting the amount
        // of locked stable
        {
            let stable_locked_amount_stat =
                accounting.get_custody_stable_locked_amount(&position.collateral_custody)?;

            stable_locked_amount_stat.locked_amount += position.locked_amount;
        }

        {
            // Check that the custody can take more short -  0 value means no limitation
            if self.pricing.max_cumulative_short_position_size_usd > 0 {
                let total_amount_shorted = accounting.get_total_shorted_amount_usd();

                msg!("total_amount_shorted {:?}", total_amount_shorted);
                msg!(
                    "max amount {:?}",
                    self.pricing.max_cumulative_short_position_size_usd
                );

                require!(
                    accounting.get_total_shorted_amount_usd()
                        <= self.pricing.max_cumulative_short_position_size_usd,
                    AdrenaError::MaxCumulativeShortPositionSizeLimit
                );
            }
        }

        // Update stats of the collateral custody (the stablecoin custody)
        {
            // Calculate interests that have accumulated for the collateral custody since last snapshot + add the new position unrealized interest
            let interest_usd = {
                let collective_position =
                    collateral_custody.get_collective_position(position.get_side())?;

                // Get the interest amount for the collective position + the new one (if any unrealized interest)
                collateral_custody.get_interest_amount_usd(&collective_position, current_time)?
                    + position.unrealized_interest_usd
            };

            let accounting_stablecoin_custody = &mut collateral_custody.short_positions;

            accounting_stablecoin_custody.cumulative_interest_usd += interest_usd;
            accounting_stablecoin_custody.cumulative_interest_snapshot =
                position.cumulative_interest_snapshot;

            accounting_stablecoin_custody.open_positions += 1;

            accounting_stablecoin_custody.locked_amount += position.locked_amount;

            accounting_stablecoin_custody.borrow_size_usd += position.borrow_size_usd;

            // Enforce limits
            if collateral_custody.pricing.max_position_locked_usd > 0 {
                let locked_amount_usd = collateral_token_price
                    .get_asset_amount_usd(position.locked_amount, collateral_custody.decimals)?;

                require!(
                    locked_amount_usd <= collateral_custody.pricing.max_position_locked_usd,
                    AdrenaError::PositionAmountLimit
                );
            }
        }

        // Update weight and quantity
        {
            let position_size_usd = math::scale_to_exponent(
                position.size_usd,
                -(Cortex::USD_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?;

            let quantity = (position_size_usd as u128 * Cortex::BPS_POWER) / position.price as u128;

            accounting.weighted_price += position.price as u128 * quantity;
            accounting.total_quantity += quantity;
        }

        Ok(())
    }

    pub fn update_accounting_after_open_position_long(
        &mut self,
        position: &Position,
        token_price: &OraclePrice,
        current_time: i64,
    ) -> Result<()> {
        // Calculate interests that have accumulated for the collateral custody since last snapshot + add the new position unrealized interest
        let interest_usd = {
            let collective_position = self.get_collective_position(position.get_side())?;

            self.get_interest_amount_usd(&collective_position, current_time)?
                + position.unrealized_interest_usd
        };

        let accounting = &mut self.long_positions;

        accounting.open_positions += 1;
        accounting.size_usd += position.size_usd;
        accounting.exit_fee_usd += position.exit_fee_usd;

        accounting.cumulative_interest_usd += interest_usd;

        accounting.cumulative_interest_snapshot = position.cumulative_interest_snapshot;

        accounting.locked_amount += position.locked_amount;
        accounting.borrow_size_usd += position.borrow_size_usd;

        accounting.collateral_usd += position.collateral_usd;

        // Enforce limits
        if self.pricing.max_position_locked_usd > 0 {
            let locked_amount_usd =
                token_price.get_asset_amount_usd(position.locked_amount, self.decimals)?;

            require!(
                locked_amount_usd <= self.pricing.max_position_locked_usd,
                AdrenaError::PositionAmountLimit
            );
        }

        // Update weight and quantity
        {
            let position_size_usd = math::scale_to_exponent(
                position.size_usd,
                -(Cortex::USD_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?;

            let quantity = (position_size_usd as u128 * Cortex::BPS_POWER) / position.price as u128;

            accounting.weighted_price += position.price as u128 * quantity;

            accounting.total_quantity += quantity;
        }

        Ok(())
    }

    pub fn update_accounting_after_remove_position_short(
        &mut self,
        position: &Position,
        current_time: i64,
        collateral_custody: &mut Custody,
    ) -> Result<()> {
        let accounting = &mut self.short_positions;

        accounting.exit_fee_usd -= position.exit_fee_usd;

        // Update the accounting of the collateral custody (the stablecoin custody)
        {
            // To be able to calculate PnL properly, needs to store on custody stats the amount
            // of locked stable
            {
                let stable_locked_amount_stat =
                    accounting.get_custody_stable_locked_amount(&position.collateral_custody)?;

                stable_locked_amount_stat.locked_amount -= position.locked_amount;
            }

            let cumulative_interest_snapshot =
                collateral_custody.get_cumulative_interest(current_time)?;

            // Calculate interests that have accumulated for the collateral custody since last snapshot
            let interest_usd = {
                let collective_position =
                    collateral_custody.get_collective_position(position.get_side())?;

                collateral_custody.get_interest_amount_usd(&collective_position, current_time)?
            };

            let position_interest_usd = collateral_custody
                .get_interest_amount_usd(position, current_time)?
                + position.unrealized_interest_usd;

            let accounting_stablecoin_custody = &mut collateral_custody.short_positions;

            // If it's the last position, reset all fields
            if accounting_stablecoin_custody.open_positions == 1 {
                *accounting_stablecoin_custody = PositionsAccounting::default();
            } else {
                accounting_stablecoin_custody.cumulative_interest_usd += interest_usd;

                // For accounting, consider all position interests being paid upon closing
                accounting_stablecoin_custody.cumulative_interest_usd =
                    accounting_stablecoin_custody
                        .cumulative_interest_usd
                        .saturating_sub(position_interest_usd);

                accounting_stablecoin_custody.cumulative_interest_snapshot =
                    cumulative_interest_snapshot.into();

                accounting_stablecoin_custody.open_positions -= 1;

                accounting_stablecoin_custody.borrow_size_usd -= position.borrow_size_usd;

                accounting_stablecoin_custody.locked_amount -= position.locked_amount;
            }
        }

        // If it's the last position, reset all fields
        if accounting.open_positions == 1 {
            *accounting = PositionsAccounting::default();

            return Ok(());
        }

        accounting.open_positions -= 1;
        accounting.size_usd -= position.size_usd;

        // Update weight and quantity
        {
            let position_size_usd = math::scale_to_exponent(
                position.size_usd,
                -(Cortex::USD_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?;

            let quantity = (position_size_usd as u128 * Cortex::BPS_POWER) / position.price as u128;

            accounting.weighted_price -= position.price as u128 * quantity;

            accounting.total_quantity -= quantity;
        }

        Ok(())
    }

    pub fn update_accounting_after_remove_position_long(
        &mut self,
        position: &Position,
        current_time: i64,
    ) -> Result<()> {
        let collective_position = self.get_collective_position(position.get_side())?;

        // Calculate interests that have accumulated for the collateral custody since last snapshot
        let interest_usd = self.get_interest_amount_usd(&collective_position, current_time)?;
        let cumulative_interest_snapshot = self.get_cumulative_interest(current_time)?;

        let position_interest_usd = self.get_interest_amount_usd(position, current_time)?
            + position.unrealized_interest_usd;

        let accounting = &mut self.long_positions;

        // If it's the last position, reset all fields
        if accounting.open_positions == 1 {
            *accounting = PositionsAccounting::default();

            return Ok(());
        }

        // Update custody accounting
        {
            accounting.exit_fee_usd -= position.exit_fee_usd;

            accounting.cumulative_interest_usd += interest_usd;

            // For accounting, consider position interest being paid upon closing
            accounting.cumulative_interest_usd = accounting
                .cumulative_interest_usd
                .saturating_sub(position_interest_usd);

            accounting.cumulative_interest_snapshot = cumulative_interest_snapshot.into();

            accounting.borrow_size_usd -= position.borrow_size_usd;

            accounting.locked_amount -= position.locked_amount;

            accounting.open_positions -= 1;

            accounting.size_usd -= position.size_usd;

            accounting.collateral_usd -= position.collateral_usd;
        }

        // Update weight and quantity
        {
            let position_size_usd = math::scale_to_exponent(
                position.size_usd,
                -(Cortex::USD_DECIMALS as i32),
                -(Cortex::PRICE_DECIMALS as i32),
            )?;

            let quantity = (position_size_usd as u128 * Cortex::BPS_POWER) / position.price as u128;

            accounting.weighted_price -= position.price as u128 * quantity;

            accounting.total_quantity -= quantity;
        }

        Ok(())
    }
}

// Implements Add/Sub for structs
impl std::ops::Add for FeesStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            swap_usd: self.swap_usd + rhs.swap_usd,
            add_liquidity_usd: self.add_liquidity_usd + rhs.add_liquidity_usd,
            remove_liquidity_usd: self.remove_liquidity_usd + rhs.remove_liquidity_usd,
            close_position_usd: self.close_position_usd + rhs.close_position_usd,
            liquidation_usd: self.liquidation_usd + rhs.liquidation_usd,
            borrow_usd: self.borrow_usd + rhs.borrow_usd,
        }
    }
}

impl std::ops::Sub for FeesStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            swap_usd: self.swap_usd - rhs.swap_usd,
            add_liquidity_usd: self.add_liquidity_usd - rhs.add_liquidity_usd,
            remove_liquidity_usd: self.remove_liquidity_usd - rhs.remove_liquidity_usd,
            close_position_usd: self.close_position_usd - rhs.close_position_usd,
            liquidation_usd: self.liquidation_usd - rhs.liquidation_usd,
            borrow_usd: self.borrow_usd - rhs.borrow_usd,
        }
    }
}

impl std::ops::Add for VolumeStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            swap_usd: self.swap_usd + rhs.swap_usd,
            add_liquidity_usd: self.add_liquidity_usd + rhs.add_liquidity_usd,
            remove_liquidity_usd: self.remove_liquidity_usd + rhs.remove_liquidity_usd,
            open_position_usd: self.open_position_usd + rhs.open_position_usd,
            close_position_usd: self.close_position_usd + rhs.close_position_usd,
            liquidation_usd: self.liquidation_usd + rhs.liquidation_usd,
        }
    }
}

impl std::ops::Sub for VolumeStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            swap_usd: self.swap_usd - rhs.swap_usd,
            add_liquidity_usd: self.add_liquidity_usd - rhs.add_liquidity_usd,
            remove_liquidity_usd: self.remove_liquidity_usd - rhs.remove_liquidity_usd,
            open_position_usd: self.open_position_usd - rhs.open_position_usd,
            close_position_usd: self.close_position_usd - rhs.close_position_usd,
            liquidation_usd: self.liquidation_usd - rhs.liquidation_usd,
        }
    }
}

impl std::ops::Add for Assets {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            collateral: self.collateral + rhs.collateral,
            owned: self.owned + rhs.owned,
            locked: self.locked + rhs.locked,
        }
    }
}

impl std::ops::Sub for Assets {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            collateral: self.collateral - rhs.collateral,
            owned: self.owned - rhs.owned,
            locked: self.locked - rhs.locked,
        }
    }
}

impl std::ops::Add for PricingParams {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            max_initial_leverage: self.max_initial_leverage + rhs.max_initial_leverage,
            max_leverage: self.max_leverage + rhs.max_leverage,
            max_position_locked_usd: self.max_position_locked_usd + rhs.max_position_locked_usd,
            max_cumulative_short_position_size_usd: self.max_cumulative_short_position_size_usd
                + rhs.max_cumulative_short_position_size_usd,
        }
    }
}

impl std::ops::Sub for PricingParams {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            max_initial_leverage: self.max_initial_leverage - rhs.max_initial_leverage,
            max_leverage: self.max_leverage - rhs.max_leverage,
            max_position_locked_usd: self.max_position_locked_usd - rhs.max_position_locked_usd,
            max_cumulative_short_position_size_usd: self.max_cumulative_short_position_size_usd
                - rhs.max_cumulative_short_position_size_usd,
        }
    }
}

impl std::ops::Add for BorrowRateParams {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            max_hourly_borrow_interest_rate: self.max_hourly_borrow_interest_rate
                + rhs.max_hourly_borrow_interest_rate,
        }
    }
}

impl std::ops::Sub for BorrowRateParams {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            max_hourly_borrow_interest_rate: self.max_hourly_borrow_interest_rate
                - rhs.max_hourly_borrow_interest_rate,
        }
    }
}

impl std::ops::Add for TradeStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            profit_usd: self.profit_usd + rhs.profit_usd,
            loss_usd: self.loss_usd + rhs.loss_usd,
            oi_long_usd: self.oi_long_usd + rhs.oi_long_usd,
            oi_short_usd: self.oi_short_usd + rhs.oi_short_usd,
        }
    }
}

impl std::ops::Sub for TradeStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            profit_usd: self.profit_usd - rhs.profit_usd,
            loss_usd: self.loss_usd - rhs.loss_usd,
            oi_long_usd: self.oi_long_usd - rhs.oi_long_usd,
            oi_short_usd: self.oi_short_usd - rhs.oi_short_usd,
        }
    }
}

impl std::ops::Add for BorrowRateState {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            current_rate: self.current_rate + rhs.current_rate,
            last_update: self.last_update + rhs.last_update,
            cumulative_interest: self.cumulative_interest + rhs.cumulative_interest,
        }
    }
}

impl std::ops::Sub for BorrowRateState {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            current_rate: self.current_rate - rhs.current_rate,
            last_update: self.last_update - rhs.last_update,
            cumulative_interest: self.cumulative_interest - rhs.cumulative_interest,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_fixture() -> Custody {
        let assets = Assets {
            owned: 1000,
            locked: 500,
            ..Assets::default()
        };

        let borrow_rate = BorrowRateParams {
            max_hourly_borrow_interest_rate: 100_000, // 0.01%
        };

        Custody {
            decimals: 5,
            assets,
            borrow_rate,
            ..Custody::default()
        }
    }

    #[test]
    fn test_update_borrow_rate() {
        let mut custody = get_fixture();

        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: U128Split::from(0),
                last_update: 3600
            }
        );

        custody.update_borrow_rate(5400).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: U128Split::from(25000),
                last_update: 5400
            }
        );

        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: U128Split::from(50000),
                last_update: 7200
            }
        );

        let mut custody = get_fixture();
        custody.update_borrow_rate(3600).unwrap();
        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: U128Split::from(50000),
                last_update: 7200
            }
        );

        let mut custody = get_fixture();
        custody.assets.locked = 0;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 0,
                cumulative_interest: U128Split::from(0),
                last_update: 3600
            }
        );

        let mut custody = get_fixture();
        custody.assets.locked = 800;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 80_000);

        let mut custody = get_fixture();
        custody.assets.locked = 900;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 90_000);

        custody.update_borrow_rate(5400).unwrap();
        assert_eq!(
            custody.borrow_rate_state.cumulative_interest,
            U128Split::from(45_000)
        );

        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(
            custody.borrow_rate_state.cumulative_interest,
            U128Split::from(90_000)
        );

        custody.assets.locked = 500;

        custody.update_borrow_rate(10800).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 50_000);
        assert_eq!(
            custody.borrow_rate_state.cumulative_interest,
            U128Split::from(180_000)
        );

        custody.update_borrow_rate(14400).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 50_000);
        assert_eq!(
            custody.borrow_rate_state.cumulative_interest,
            U128Split::from(230_000)
        );

        let mut custody = get_fixture();
        custody.assets.locked = 1000;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 100_000);

        let mut custody = get_fixture();
        custody.assets.locked = 1;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 100);

        let mut custody = get_fixture();
        custody.assets.locked = 999;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 99_900);
    }
}
