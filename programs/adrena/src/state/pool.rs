use {
    super::{
        cortex::ProfitAndLoss,
        custody::MAX_STABLE_CUSTODY,
        oracle::{Oracle, ORACLE_EXPONENT_SCALE, ORACLE_PRICE_SCALE},
    },
    crate::{
        error::AdrenaError,
        math,
        state::{
            cortex::Cortex,
            custody::{Custody, MIN_INITIAL_LEVERAGE},
            oracle::OraclePrice,
            position::{Position, Side},
        },
        utils::{limited_string::LimitedString, u128_split::U128Split},
    },
    anchor_lang::prelude::*,
    bytemuck::{Pod, Zeroable},
};

// When the genesis lock ends for main_pool on mainnet
// It's very specific to the mainnet, and good for one time use to make ALP fully liquid
pub const FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP: i64 = 1742385600;

pub struct StableCustodyInfo {
    pub custody: Pubkey,
    pub token_price: OraclePrice,
    pub decimals: u8,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Zeroable, Pod,
)]
#[repr(C)]
pub struct TokenRatios {
    pub target: u16,
    pub min: u16,
    pub max: u16,
    pub _padding: [u8; 2],
}

// The pool go though different states during its lifecycle
// - GenesisLiquidity: The pool is created and the genesis liquidity is added.
// - Idle: The pool is in wait mode. Do not accept new liquidity.
// - Active: The pool is active and accepts new liquidity.
//
// Regular flow is:
// The pool is initially in GenesisLiquidity state.
// Then the state is changed to Idle after the genesis period ends.
// Then the state is manually changed to Active to add more liquidity (nominal state for the pool).
// The state can manually be changed back to Idle to pause the add of liquidity.
//
#[derive(PartialEq, Copy, Clone, Default, Debug)]
#[repr(u8)]
pub enum PoolLiquidityState {
    #[default]
    GenesisLiquidity = 0,
    Idle = 1,
    Active = 2,
}

impl From<PoolLiquidityState> for u8 {
    fn from(val: PoolLiquidityState) -> Self {
        match val {
            PoolLiquidityState::GenesisLiquidity => 0,
            PoolLiquidityState::Idle => 1,
            PoolLiquidityState::Active => 2,
        }
    }
}

impl TryFrom<u8> for PoolLiquidityState {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => PoolLiquidityState::GenesisLiquidity,
            1 => PoolLiquidityState::Idle,
            2 => PoolLiquidityState::Active,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidPoolLiquidityState)?,
        })
    }
}

#[derive(Debug)]
pub struct ExitPositionNumbers {
    pub close_amount: u64,
    pub exit_fee: u64,
    pub exit_fee_usd: u64,
    pub borrow_fee: u64,
    pub borrow_fee_usd: u64,
    pub profit_usd: u64,
    pub loss_usd: u64,
    // The amount of USD that the user can't pay for the fees
    pub deficit_fee_usd: u64,
    // The amount of loss in USD the user can't cover - net loss for the pool
    pub deficit_pool_usd: u64,
    pub total_fee: u64,     // borrow_fee + exit_fee
    pub total_fee_usd: u64, // borrow_fee_usd + exit_fee_usd
}

pub const MAX_CUSTODIES: usize = 8;

// For which type of operation the leverage is checked
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LeverageCheckType {
    Initial,
    AddCollateral,
    RemoveCollateral,
    IncreasePosition,
    Liquidate,
}

#[account(zero_copy)]
#[derive(Default, Debug)]
#[repr(C)]
pub struct Pool {
    pub bump: u8,
    pub lp_token_bump: u8,
    pub nb_stable_custody: u8,
    //
    pub initialized: u8, // 0 = false, 1 = true
    //
    // Permissions (applicable to all custodies)
    //
    // If false, make positions readonly/closeonly
    pub allow_trade: u8,
    pub allow_swap: u8,
    //
    pub liquidity_state: u8, // PoolLiquidityState,
    //
    pub registered_custody_count: u8,
    //
    pub name: LimitedString,
    //
    pub custodies: [Pubkey; MAX_CUSTODIES],
    // Keep track of fees debt
    pub fees_debt_usd: u64, // Doesn't include the referrers_fee_debt_usd
    pub referrers_fee_debt_usd: u64,
    //
    // Keep a stat about how much referral fees have been generated from all time
    pub cumulative_referrer_fee_usd: u64,
    pub lp_token_price_usd: u64,
    //
    pub whitelisted_swapper: Pubkey,
    pub ratios: [TokenRatios; MAX_CUSTODIES],
    pub last_aum_and_lp_token_price_usd_update: i64,
    // Unique ID counter for limit orders (incremented with wrapping add, looping)
    pub unique_limit_order_id_counter: u64,
    pub aum_usd: U128Split,
    //
    pub inception_time: i64,
    //
    // Ideal max AUM value for the pool
    // It's a soft limit considering the assets in the pool can increase of value,
    // thus making the AUM grow higher than the limit
    pub aum_soft_cap_usd: u64,
}

impl TokenRatios {
    pub fn validate(&self) -> bool {
        (self.target as u128) <= Cortex::BPS_POWER
            && (self.min as u128) <= Cortex::BPS_POWER
            && (self.max as u128) <= Cortex::BPS_POWER
            && self.min <= self.target
            && self.target <= self.max
    }
}

/// All returned prices are scaled to PRICE_DECIMALS.
/// All returned amounts are scaled to corresponding custody decimals.
impl Pool {
    // 8 bytes for anchor discriminator
    pub const LEN: usize = 8 + std::mem::size_of::<Pool>();

    pub fn get_liquidity_state(&self) -> PoolLiquidityState {
        // Consider the value inside of the struct always good
        PoolLiquidityState::try_from(self.liquidity_state).unwrap()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized != 0
    }

    pub fn is_trade_allowed(&self) -> bool {
        self.allow_trade != 0 && self.liquidity_state == PoolLiquidityState::Active as u8
    }

    pub fn is_swap_allowed(&self) -> bool {
        self.allow_swap != 0
            && (self.liquidity_state == PoolLiquidityState::Idle as u8
                || self.liquidity_state == PoolLiquidityState::Active as u8)
    }

    pub fn validate(&self) -> bool {
        for ratio in &self.ratios {
            if !ratio.validate() {
                return false;
            }
        }

        if self.nb_stable_custody as usize > MAX_STABLE_CUSTODY {
            return false;
        }

        // check target ratios add up to 1
        if self.registered_custody_count > 0
            && self
                .ratios
                .iter()
                .map(|&x| (x.target as u128))
                .sum::<u128>()
                != Cortex::BPS_POWER
        {
            return false;
        }

        // Check custodies are unique
        {
            for i in 0..self.custodies.len() {
                let custody = self.custodies[i];
                if custody == Pubkey::default() {
                    // Skip empty/default slots
                    continue;
                }

                // Check against all subsequent elements
                for j in i + 1..self.custodies.len() {
                    if custody == self.custodies[j] {
                        return false; // Duplicate found
                    }
                }
            }
        }

        true
    }

    // Utility function used to avoid dealing with blank spots in custodies array
    pub fn get_custodies(&self) -> Vec<Pubkey> {
        let mut custodies: Vec<Pubkey> = vec![];

        for &custody in &self.custodies {
            if custody != Pubkey::default() {
                custodies.push(custody);
            }
        }

        custodies
    }

    /// Adds a custody key to the pool, returns an error if no spots are available.
    pub fn add_custody(&mut self, custody_key: Pubkey) -> Result<()> {
        let spot = self.custodies.iter().position(|p| *p == Pubkey::default());
        match spot {
            Some(index) => {
                self.custodies[index] = custody_key;
                self.registered_custody_count += 1;
                Ok(())
            }
            None => Err(AdrenaError::InvalidCustodyState.into()),
        }
    }

    /// Removes a custody key from the pool, returns an error if the key is not found.
    pub fn remove_custody(&mut self, custody_key: &Pubkey) -> Result<()> {
        let token_id = self.custodies.iter().position(|p| *p == *custody_key);
        match token_id {
            Some(index) => {
                self.custodies[index] = Pubkey::default();
                self.registered_custody_count -= 1;
                Ok(())
            }
            None => Err(AdrenaError::InvalidCustodyState.into()),
        }
    }

    pub fn get_token_id(&self, custody: &Pubkey) -> Result<usize> {
        self.custodies
            .iter()
            .position(|&k| k == *custody)
            .ok_or_else(|| AdrenaError::UnsupportedToken.into())
    }

    pub fn get_exit_fee(&self, size: u64, custody: &Custody) -> Result<u64> {
        Self::get_fee_amount(custody.fees.close_position, size)
    }

    // Close amount are the tokens to send back to the user when closing the position
    #[allow(clippy::too_many_arguments)]
    pub fn get_exit_position_numbers(
        &self,
        position: &Position,
        token_trade_price: &OraclePrice,
        collateral_token_price: &OraclePrice,
        collateral_custody: &Custody,
        current_time: i64,
        liquidation: bool,
    ) -> Result<ExitPositionNumbers> {
        let pnl = self.get_pnl_usd(
            position,
            token_trade_price,
            collateral_token_price,
            collateral_custody,
            current_time,
            liquidation,
        )?;

        let (close_amount_usd, exit_fee_usd, borrow_fee_usd, deficit_fee_usd, deficit_pool_usd) =
            (|| -> (u64, u64, u64, u64, u64) {
                // The user is in profit
                if pnl.profit_usd > 0 {
                    return (
                        position.collateral_usd + pnl.profit_usd,
                        pnl.exit_fee_usd,
                        pnl.borrow_fee_usd,
                        0,
                        0,
                    );
                }

                // The user is losing money, but the loss is covered by the collateral
                if pnl.loss_usd <= position.collateral_usd {
                    // The user get back the collateral minus the loss
                    return (
                        position.collateral_usd - pnl.loss_usd,
                        pnl.exit_fee_usd,
                        pnl.borrow_fee_usd,
                        0,
                        0,
                    );
                }

                // The user is in deficit and there is not enough money to pay for the loss (exit_fee + borrow_fee + price difference)
                let total_deficit_usd = pnl.loss_usd - position.collateral_usd;

                // The deficit is smaller than the fees, meaning the pool isn't losing money from the trade, but can't get its fees fully paid
                if total_deficit_usd <= pnl.exit_fee_usd + pnl.borrow_fee_usd {
                    //
                    // Note: the borrow_fee get paid first. If there are less money available, it's the exit_fee that get reduced in priority
                    //
                    if total_deficit_usd > pnl.exit_fee_usd {
                        let mut d = total_deficit_usd;

                        // Means the exit_fee can't get paid at all
                        d -= pnl.exit_fee_usd;

                        let updated_exit_fee_usd = 0;
                        let updated_borrow_fee_usd = pnl.borrow_fee_usd - d;

                        return (
                            0,
                            updated_exit_fee_usd,
                            updated_borrow_fee_usd,
                            total_deficit_usd,
                            0,
                        );
                    }

                    // Means that the exit_fee can get paid partially and the borrow_fee can get paid fully
                    let updated_exit_fee_usd = pnl.exit_fee_usd - total_deficit_usd;
                    let updated_borrow_fee_usd = pnl.borrow_fee_usd;

                    return (
                        0,
                        updated_exit_fee_usd,
                        updated_borrow_fee_usd,
                        total_deficit_usd,
                        0,
                    );
                }

                // The deficit is bigger than the fee, meaning the pool is losing money. In that state, we don't take fees to limit the loss.
                (
                    0,
                    0,
                    0,
                    pnl.exit_fee_usd + pnl.borrow_fee_usd,
                    total_deficit_usd - (pnl.exit_fee_usd + pnl.borrow_fee_usd),
                )
            })();

        let close_amount = collateral_token_price
            .high()
            .get_token_amount(close_amount_usd, collateral_custody.decimals)?;
        let exit_fee =
            collateral_token_price.get_token_amount(exit_fee_usd, collateral_custody.decimals)?;
        let borrow_fee =
            collateral_token_price.get_token_amount(borrow_fee_usd, collateral_custody.decimals)?;

        // /!\ Extra safety measure
        // The maximum amount that can be taken from the pool to pay the user without going into deficit
        let max_amount = (position.locked_amount + position.collateral_amount)
            .saturating_sub(pnl.exit_fee + borrow_fee);

        Ok(ExitPositionNumbers {
            close_amount: std::cmp::min(close_amount, max_amount),
            exit_fee,
            borrow_fee,
            profit_usd: pnl.profit_usd,
            loss_usd: pnl.loss_usd,
            exit_fee_usd,
            borrow_fee_usd,
            deficit_fee_usd,
            deficit_pool_usd,
            total_fee: exit_fee + borrow_fee,
            total_fee_usd: exit_fee_usd + borrow_fee_usd,
        })
    }

    pub fn get_swap_price(
        &self,
        token_in_price: &OraclePrice,
        token_out_price: &OraclePrice,
    ) -> Result<(u64, i32)> {
        let base = token_in_price.normalize()?;
        let other = token_out_price.normalize()?;

        let price =
            math::checked_as_u64((base.price as u128 * ORACLE_PRICE_SCALE) / other.price as u128)?;
        let exponent = (base.exponent + ORACLE_EXPONENT_SCALE) - other.exponent;

        Ok((price, exponent))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_swap_amount(
        &self,
        token_in_price: &OraclePrice,
        token_out_price: &OraclePrice,
        custody_in: &Custody,
        custody_out: &Custody,
        amount_in: u64,
    ) -> Result<u64> {
        let swap_price = self.get_swap_price(token_in_price, token_out_price)?;

        math::checked_decimal_mul(
            amount_in,
            -(custody_in.decimals as i32),
            swap_price.0,
            swap_price.1,
            -(custody_out.decimals as i32),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_swap_in_fees(
        &self,
        amount_in: u64,
        custody_in: &Custody,
        custody_out: &Custody,
    ) -> Result<u64> {
        let stable_swap = custody_in.is_stable() && custody_out.is_stable();

        let swap_in_fee = Self::get_fee_amount(
            if stable_swap {
                custody_in.fees.stable_swap_in
            } else {
                custody_in.fees.swap_in
            },
            amount_in,
        )?;

        msg!("swap_in_fee: {}", swap_in_fee);

        Ok(swap_in_fee)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_swap_out_fees(
        &self,
        amount_out: u64,
        custody_in: &Custody,
        custody_out: &Custody,
    ) -> Result<u64> {
        let stable_swap = custody_in.is_stable() && custody_out.is_stable();

        let swap_out_fee = Self::get_fee_amount(
            if stable_swap {
                custody_out.fees.stable_swap_out
            } else {
                custody_out.fees.swap_out
            },
            amount_out,
        )?;

        msg!("swap_out_fee: {}", swap_out_fee);

        Ok(swap_out_fee)
    }

    pub fn get_add_liquidity_fee(&self, amount: u64, custody: &Custody) -> Result<u64> {
        Self::get_fee_amount(custody.fees.add_liquidity, amount)
    }

    pub fn get_remove_liquidity_fee(&self, amount: u64, custody: &Custody) -> Result<u64> {
        Self::get_fee_amount(custody.fees.remove_liquidity, amount)
    }

    pub fn get_liquidation_fee(&self, size: u64, custody: &Custody) -> Result<u64> {
        Self::get_fee_amount(custody.fees.liquidation, size)
    }

    pub fn check_token_ratio(
        &self,
        token_id: usize,
        amount_add: u64,
        amount_remove: u64,
        custody: &Custody,
        token_price: &OraclePrice,
    ) -> Result<bool> {
        let new_ratio = self.get_new_ratio(amount_add, amount_remove, custody, token_price)?;

        if new_ratio < self.ratios[token_id].min {
            Ok(new_ratio >= self.get_current_ratio(custody, token_price)?)
        } else if new_ratio > self.ratios[token_id].max {
            Ok(new_ratio <= self.get_current_ratio(custody, token_price)?)
        } else {
            Ok(true)
        }
    }

    pub fn check_available_amount(&self, amount: u64, custody: &Custody) -> Result<bool> {
        let available_amount =
            (custody.assets.owned + custody.assets.collateral) - custody.assets.locked;

        Ok(available_amount >= amount)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_leverage(
        &self,
        position: &Position,
        token_trade_price: &OraclePrice,
        collateral_token_price: &OraclePrice,
        collateral_custody: &Custody,
        current_time: i64,
        // true: calculate the PnL with liquidation_fee_usd
        // false: calculate the PnL with exit_fee_usd
        liquidation: bool,
    ) -> Result<u64> {
        // Do not accept 0 price
        if position.price == 0 {
            return Ok(u64::MAX);
        }

        let pnl = self.get_pnl_usd(
            position,
            token_trade_price,
            collateral_token_price,
            collateral_custody,
            current_time,
            liquidation,
        )?;

        let current_margin_usd = (|| {
            // Nor profits or losses
            if pnl.profit_usd == 0 && pnl.loss_usd == 0 {
                return position.collateral_usd;
            }

            // Profit
            if pnl.profit_usd > 0 {
                return position.collateral_usd + pnl.profit_usd;
            }

            // Partial loss
            if pnl.loss_usd <= position.collateral_usd {
                return position.collateral_usd - pnl.loss_usd;
            }

            // Total loss
            0
        })();

        if current_margin_usd > 0 {
            math::checked_as_u64(
                (position.size_usd as u128 * Cortex::BPS_POWER) / current_margin_usd as u128,
            )
        } else {
            Ok(u64::MAX)
        }
    }

    /// Checks if leverage is within the limits (and return the value for events)
    #[allow(clippy::too_many_arguments)]
    pub fn check_leverage(
        &self,
        position: &Position,
        token_trade_price: &OraclePrice,
        custody: &Custody,
        collateral_token_price: &OraclePrice,
        collateral_custody: &Custody,
        current_time: i64,
        check_type: LeverageCheckType,
    ) -> Result<u64> {
        // Idea is to check the leverage considering the highest fee when not creating a new position
        // Position should always be able to pay liquidation fee
        let use_liquidation_fee_usd_for_pnl_calculation = check_type
            == LeverageCheckType::Liquidate
            && position.liquidation_fee_usd > position.exit_fee_usd;

        let leverage = self.get_leverage(
            position,
            token_trade_price,
            collateral_token_price,
            collateral_custody,
            current_time,
            use_liquidation_fee_usd_for_pnl_calculation,
        )?;

        msg!("leverage: {}", leverage);

        // In all case, leverage should not exceed max leverage
        require!(
            leverage <= custody.pricing.max_leverage as u64,
            AdrenaError::MaxLeverage
        );

        match check_type {
            LeverageCheckType::Initial
            | LeverageCheckType::RemoveCollateral
            | LeverageCheckType::IncreasePosition => {
                // When creating a position, withdrawing collateral or increasing a position, leverage should be at least the min initial leverage
                require!(
                    leverage >= MIN_INITIAL_LEVERAGE as u64,
                    AdrenaError::MinLeverage
                );
                // And at most the max initial leverage
                require!(
                    leverage <= custody.pricing.max_initial_leverage as u64,
                    AdrenaError::MaxLeverage
                );
            }
            LeverageCheckType::AddCollateral => {
                // When adding collateral, leverage should be at least the min initial leverage and at most the max leverage (but checked for all cases above)
                require!(
                    leverage >= MIN_INITIAL_LEVERAGE as u64,
                    AdrenaError::MinLeverage
                );
            }
            LeverageCheckType::Liquidate => {
                // No specific checks, this function could use a refactor as it's used for different purpose, but in case of liquidation,
                // we want to check the max leverage usually afterward
            }
        }

        Ok(leverage)
    }

    pub fn get_liquidation_price(
        &self,
        position: &Position,
        custody: &Custody,
        collateral_custody: &Custody,
        current_time: i64,
    ) -> Result<u64> {
        // liq_price = pos_price +- (collateral + unreal_profit - unreal_loss - exit_fee - interest - size/max_leverage) * pos_price / size

        if position.size_usd == 0 || position.price == 0 {
            return Ok(0);
        }

        let total_unrealized_interest_usd = collateral_custody
            .get_interest_amount_usd(position, current_time)?
            + position.unrealized_interest_usd;
        let unrealized_loss_usd = position.liquidation_fee_usd + total_unrealized_interest_usd;

        let mut max_loss_usd = math::checked_as_u64(
            (position.size_usd as u128 * Cortex::BPS_POWER) / custody.pricing.max_leverage as u128,
        )?;

        max_loss_usd += unrealized_loss_usd;

        let margin_usd = position.collateral_usd;

        let max_price_diff = if max_loss_usd >= margin_usd {
            max_loss_usd - margin_usd
        } else {
            margin_usd - max_loss_usd
        };

        let max_price_diff = math::scale_to_exponent(
            max_price_diff,
            -(Cortex::USD_DECIMALS as i32),
            -(Cortex::PRICE_DECIMALS as i32),
        )?;

        let position_size_usd = math::scale_to_exponent(
            position.size_usd,
            -(Cortex::USD_DECIMALS as i32),
            -(Cortex::PRICE_DECIMALS as i32),
        )?;

        let max_price_diff = math::checked_as_u64(
            (max_price_diff as u128 * position.price as u128) / position_size_usd as u128,
        )?;

        if position.get_side() == Side::Long {
            if max_loss_usd >= margin_usd {
                Ok(position.price + max_price_diff)
            } else if position.price > max_price_diff {
                Ok(position.price - max_price_diff)
            } else {
                Ok(0)
            }
        } else if max_loss_usd >= margin_usd {
            if position.price > max_price_diff {
                Ok(position.price - max_price_diff)
            } else {
                Ok(0)
            }
        } else {
            Ok(position.price + max_price_diff)
        }
    }

    // Note: PnL is a unrealised PnL
    // Note that the PnL is an estimation and can be different when the position is closed due to exact fees not known until actual close (this estimation is calculated conservatively)
    #[allow(clippy::too_many_arguments)]
    pub fn get_pnl_usd(
        &self,
        position: &Position,
        token_trade_price: &OraclePrice,
        collateral_token_price: &OraclePrice,
        collateral_custody: &Custody,
        current_time: i64,
        liquidation: bool,
    ) -> Result<ProfitAndLoss> {
        if position.size_usd == 0 || position.price == 0 {
            return Ok(ProfitAndLoss::default());
        }

        // Use High/Low price to protect the pool
        let exit_price = match Side::try_from(position.side)? {
            Side::Long => token_trade_price.price,
            Side::Short => token_trade_price.price,
            Side::None => return Err(AdrenaError::InvalidPositionState.into()),
        };

        let exit_fee_usd: u64 = if liquidation {
            position.liquidation_fee_usd
        } else {
            position.exit_fee_usd
        };

        // Marginal but uses low price for safety
        let exit_fee = collateral_token_price
            .low()
            .get_token_amount(exit_fee_usd, collateral_custody.decimals)?;

        let total_unrealized_interest_usd = collateral_custody
            .get_interest_amount_usd(position, current_time)?
            + position.unrealized_interest_usd;

        let unrealized_loss_usd = exit_fee_usd + total_unrealized_interest_usd;

        let (price_diff_profit, price_diff_loss) = if position.get_side() == Side::Long {
            if exit_price > position.price {
                (exit_price - position.price, 0u64)
            } else {
                (0u64, position.price - exit_price)
            }
        } else if exit_price < position.price {
            (position.price - exit_price, 0u64)
        } else {
            (0u64, exit_price - position.price)
        };

        if price_diff_profit > 0 {
            let potential_profit_usd = math::checked_as_u64(
                (position.size_usd as u128 * price_diff_profit as u128) / position.price as u128,
            )?;

            if potential_profit_usd >= (unrealized_loss_usd + position.paid_interest_usd) {
                let cur_profit_usd =
                    potential_profit_usd - (unrealized_loss_usd + position.paid_interest_usd);

                let max_profit_usd = if current_time <= position.open_time {
                    0
                } else {
                    collateral_token_price
                        .low()
                        .get_asset_amount_usd(position.locked_amount, collateral_custody.decimals)?
                };

                Ok(ProfitAndLoss {
                    profit_usd: std::cmp::min(max_profit_usd, cur_profit_usd),
                    loss_usd: 0u64,
                    exit_fee,
                    exit_fee_usd,
                    borrow_fee_usd: total_unrealized_interest_usd + position.paid_interest_usd,
                })
            } else {
                Ok(ProfitAndLoss {
                    profit_usd: 0u64,
                    loss_usd: (unrealized_loss_usd + position.paid_interest_usd)
                        - potential_profit_usd,
                    exit_fee,
                    exit_fee_usd,
                    borrow_fee_usd: total_unrealized_interest_usd + position.paid_interest_usd,
                })
            }
        } else {
            let mut potential_loss_usd = math::checked_as_u64(math::checked_ceil_div::<u128>(
                position.size_usd as u128 * price_diff_loss as u128,
                position.price as u128,
            )?)?;

            potential_loss_usd += unrealized_loss_usd + position.paid_interest_usd;

            Ok(ProfitAndLoss {
                profit_usd: 0u64,
                loss_usd: potential_loss_usd,
                exit_fee,
                exit_fee_usd,
                borrow_fee_usd: total_unrealized_interest_usd + position.paid_interest_usd,
            })
        }
    }

    pub fn get_assets_under_management_usd(
        &self,
        oracle: &Oracle,
        accounts: &[AccountInfo],
        current_time: i64,
    ) -> Result<u128> {
        // Pre-load stable custodies info to calculate PnL for short positions properly
        //
        // Have to pre-load as multiple custodies cannot be loaded in memory at same time
        let custodies = self.get_custodies();

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

        // Calculate how many trade_oracle are required

        let mut pool_amount_usd: u128 = 0;
        let mut total_pnl_profits: u128 = 0;
        let mut total_pnl_losses: u128 = 0;

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

            pool_amount_usd += token_amount_usd as u128;

            if custody.is_stable() {
                let collective_position = custody.get_collective_position(Side::Short)?;
                let interest_usd = custody
                    .get_interest_amount_usd(&collective_position, current_time)?
                    + collective_position.unrealized_interest_usd;

                // Consider all new interests as part of the pool
                pool_amount_usd += interest_usd as u128;
            } else {
                // compute long aggregate unrealized pnl
                let long_pnl = self.get_pnl_usd(
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

                    for stable_locked_amount in custody.short_positions.stable_locked_amount.iter()
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

                let short_pnl = self.get_pnl_usd(
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

                            total_pnl_losses += profit_usd as u128;
                        }
                        std::cmp::Ordering::Greater => {
                            // The pool is losing money on collateral
                            let loss_usd = previous_long_positions_collateral_usd
                                - current_long_positions_collateral_usd;

                            total_pnl_profits += loss_usd as u128;
                        }
                    }
                }

                // Store PnLs
                total_pnl_profits += long_pnl.profit_usd as u128;
                total_pnl_profits += short_pnl.profit_usd as u128;

                total_pnl_losses += long_pnl.loss_usd as u128;
                total_pnl_losses += short_pnl.loss_usd as u128;
            }
        }

        // Adjust pool amount by collective profit/loss

        // Unrealized position losses are considered part of the pool (contains interests)
        pool_amount_usd += total_pnl_losses;

        // Unrealized position profits are considered out of the pool
        pool_amount_usd = pool_amount_usd.saturating_sub(total_pnl_profits);

        pool_amount_usd =
            pool_amount_usd - self.fees_debt_usd as u128 - self.referrers_fee_debt_usd as u128;

        Ok(pool_amount_usd)
    }

    pub fn get_fee_amount(fee: u16, amount: u64) -> Result<u64> {
        if fee == 0 || amount == 0 {
            return Ok(0);
        }

        math::checked_as_u64(math::checked_ceil_div::<u128>(
            amount as u128 * fee as u128,
            Cortex::BPS_POWER,
        )?)
    }

    // private helpers
    fn get_current_ratio(&self, custody: &Custody, token_price: &OraclePrice) -> Result<u16> {
        if self.aum_usd.to_u128() == 0 {
            return Ok(0);
        }

        let ratio = math::checked_as_u16(
            (token_price.get_asset_amount_usd(custody.assets.owned, custody.decimals)? as u128
                * Cortex::BPS_POWER)
                / self.aum_usd.to_u128(),
        )?;

        Ok(std::cmp::min(ratio, Cortex::BPS_POWER as u16))
    }

    fn get_new_ratio(
        &self,
        amount_add: u64,
        amount_remove: u64,
        custody: &Custody,
        token_price: &OraclePrice,
    ) -> Result<u16> {
        let (new_token_aum_usd, new_pool_aum_usd) = if amount_add > 0 && amount_remove > 0 {
            return Err(ProgramError::InvalidArgument.into());
        } else if amount_add == 0 && amount_remove == 0 {
            (
                token_price.get_asset_amount_usd(custody.assets.owned, custody.decimals)? as u128,
                self.aum_usd.to_u128(),
            )
        } else if amount_add > 0 {
            let added_aum_usd =
                token_price.get_asset_amount_usd(amount_add, custody.decimals)? as u128;

            (
                token_price
                    .get_asset_amount_usd(custody.assets.owned + amount_add, custody.decimals)?
                    as u128,
                self.aum_usd.to_u128() + added_aum_usd,
            )
        } else {
            let removed_aum_usd =
                token_price.get_asset_amount_usd(amount_remove, custody.decimals)? as u128;

            if removed_aum_usd >= self.aum_usd.to_u128() || amount_remove >= custody.assets.owned {
                (0, 0)
            } else {
                (
                    token_price.get_asset_amount_usd(
                        custody.assets.owned - amount_remove,
                        custody.decimals,
                    )? as u128,
                    self.aum_usd.to_u128() - removed_aum_usd,
                )
            }
        };

        if new_token_aum_usd == 0 || new_pool_aum_usd == 0 {
            return Ok(0);
        }

        let ratio =
            math::checked_as_u16((new_token_aum_usd * Cortex::BPS_POWER) / new_pool_aum_usd)?;

        Ok(std::cmp::min(ratio, Cortex::BPS_POWER as u16))
    }

    pub fn get_unique_limit_order_id(&mut self) -> u64 {
        let id = self.unique_limit_order_id_counter;

        self.unique_limit_order_id_counter = self.unique_limit_order_id_counter.wrapping_add(1);

        // 0 is reserved for the default value
        if id == 0 {
            return self.get_unique_limit_order_id();
        }

        id
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            state::custody::{BorrowRateParams, Fees, PricingParams},
            utils::u128_split::U128Split,
        },
    };

    fn get_fixture() -> (Pool, Custody, Position, OraclePrice) {
        let ratios = TokenRatios {
            target: 5_000,
            min: 1_000,
            max: 9_000,
            ..TokenRatios::default()
        };

        let pricing = PricingParams {
            max_initial_leverage: 1_050_000,
            max_leverage: 1_100_000,
            max_position_locked_usd: 0,
            max_cumulative_short_position_size_usd: 0,
        };

        let fees = Fees {
            swap_in: 10,
            swap_out: 10,
            stable_swap_in: 10,
            stable_swap_out: 10,
            add_liquidity: 0,
            remove_liquidity: 0,
            close_position: 10,
            liquidation: 50,
            fee_max: 100,
            ..Default::default()
        };

        let custody = Custody {
            token_account: Pubkey::default(),
            mint: Pubkey::default(),
            decimals: 9,
            oracle: LimitedString::default(),
            trade_oracle: LimitedString::default(),
            pricing,
            fees,
            allow_swap: true as u8,
            allow_trade: true as u8,
            ..Custody::default()
        };

        let collateral_usd: u64 = scale(25_000, Cortex::USD_DECIMALS);
        let leverage: u64 = 40_000; // x4

        // Use collateral and leverage to figure out the rest
        let size_usd: u64 =
            math::checked_as_u64(collateral_usd as u128 * leverage as u128 / Cortex::BPS_POWER)
                .unwrap();

        let exit_fee_usd = Pool::get_fee_amount(fees.close_position, size_usd).unwrap();
        let liquidation_fee_usd = Pool::get_fee_amount(fees.liquidation, size_usd).unwrap();

        let adjusted_collateral_usd = collateral_usd - exit_fee_usd;

        let adjusted_size_usd = math::checked_as_u64(
            adjusted_collateral_usd as u128 * leverage as u128 / Cortex::BPS_POWER,
        )
        .unwrap();

        let position = Position {
            side: Side::Long.into(),
            price: scale(25_000, Cortex::PRICE_DECIMALS),
            // x4 leverage
            size_usd: adjusted_size_usd,
            borrow_size_usd: scale(100_000, Cortex::USD_DECIMALS),
            collateral_usd,
            locked_amount: scale(4, 9),
            collateral_amount: scale(1, 9),
            exit_fee_usd,
            liquidation_fee_usd,
            ..Position::default()
        };

        let token_price = OraclePrice {
            price: 25_000_000,
            exponent: -3,
            confidence: 0,
            ..Default::default()
        }
        .scale_to_exponent(-(Cortex::PRICE_DECIMALS as i32))
        .unwrap();

        (
            Pool {
                ratios: [
                    ratios,
                    ratios,
                    TokenRatios::default(),
                    TokenRatios::default(),
                    TokenRatios::default(),
                    TokenRatios::default(),
                    TokenRatios::default(),
                    TokenRatios::default(),
                ],
                ..Default::default()
            },
            custody,
            position,
            token_price,
        )
    }

    fn scale(amount: u64, decimals: u8) -> u64 {
        amount * 10u64.pow(decimals as u32)
    }

    fn scale_f64(amount: f64, decimals: u8) -> u64 {
        math::checked_as_u64(
            math::checked_float_mul(amount, 10u64.pow(decimals as u32) as f64).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn test_get_new_ratio() {
        let (mut pool, mut custody, _position, token_price) = get_fixture();

        // add tokens to empty custody
        assert_eq!(
            scale(1, Cortex::BPS_DECIMALS) as u16,
            pool.get_new_ratio(1_000, 0, &custody, &token_price)
                .unwrap()
        );

        // remove tokens from empty custody
        assert_eq!(
            0,
            pool.get_new_ratio(0, 1_000, &custody, &token_price)
                .unwrap()
        );

        // cannot provide both add and remove
        assert!(pool
            .get_new_ratio(1_000, 1_000, &custody, &token_price)
            .is_err());

        // doesn't change ratio
        assert_eq!(0, pool.get_new_ratio(0, 0, &custody, &token_price).unwrap());

        // add value to the pool for the custody to be 50% ratio
        pool.aum_usd = U128Split::new(scale(50_000_000, Cortex::USD_DECIMALS) as u128);
        custody.assets.owned = scale(1_000, custody.decimals);

        // add tokens to go 50%+ ratio
        assert_eq!(
            5238,
            pool.get_new_ratio(scale(100, custody.decimals), 0, &custody, &token_price)
                .unwrap()
        );

        // removes tokens to get 50%- ratio
        assert_eq!(
            4_736,
            pool.get_new_ratio(0, scale(100, custody.decimals), &custody, &token_price)
                .unwrap()
        );

        // removes all tokens to get to 0% ratio
        assert_eq!(
            0,
            pool.get_new_ratio(0, scale(1_000, custody.decimals), &custody, &token_price)
                .unwrap()
        );

        // changes nothing should return same ratio
        assert_eq!(
            5_000,
            pool.get_new_ratio(0, 0, &custody, &token_price).unwrap()
        );
    }

    #[test]
    fn test_get_pnl_usd() {
        let (pool, custody, mut position, token_price) = get_fixture();

        // initial PnL at loss
        assert_eq!(
            ProfitAndLoss {
                profit_usd: 0,
                loss_usd: scale(100, Cortex::USD_DECIMALS),
                exit_fee: scale(4, Cortex::USD_DECIMALS),
                exit_fee_usd: scale(100, Cortex::USD_DECIMALS),
                borrow_fee_usd: scale(0, Cortex::USD_DECIMALS)
            },
            pool.get_pnl_usd(&position, &token_price, &token_price, &custody, 1, false)
                .unwrap()
        );

        // losing position (opening price higher than current price)
        position.price = scale(25_400, Cortex::PRICE_DECIMALS);

        assert_eq!(
            ProfitAndLoss {
                profit_usd: 0,
                loss_usd: scale_f64(1_668.503938, Cortex::USD_DECIMALS),
                exit_fee: scale(4, Cortex::USD_DECIMALS),
                exit_fee_usd: scale(100, Cortex::USD_DECIMALS),
                borrow_fee_usd: scale(0, Cortex::USD_DECIMALS)
            },
            pool.get_pnl_usd(&position, &token_price, &token_price, &custody, 1, false)
                .unwrap()
        );

        // winning position (opening price lower than current price)
        position.price = scale(24_500, Cortex::PRICE_DECIMALS);
        assert_eq!(
            ProfitAndLoss {
                profit_usd: scale_f64(1_932.653061, Cortex::USD_DECIMALS),
                loss_usd: 0,
                exit_fee: scale(4, Cortex::USD_DECIMALS),
                exit_fee_usd: scale(100, Cortex::USD_DECIMALS),
                borrow_fee_usd: scale(0, Cortex::USD_DECIMALS)
            },
            pool.get_pnl_usd(&position, &token_price, &token_price, &custody, 1, false)
                .unwrap()
        );
    }

    #[test]
    fn test_get_leverage() {
        let (pool, custody, mut position, token_price) = get_fixture();

        // default leverage
        assert_eq!(
            40_653,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        // lower price should lower leverage for long position
        position.price = scale(20_040, Cortex::PRICE_DECIMALS);
        assert_eq!(
            20_263,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        position.price = scale(15_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            10_957,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        // higher price should increase leverage for long position
        position.price = scale(27_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            5_8170,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        position.price = scale(32_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            367_188,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        // leverage out of limit when bad price
        position.price = 0;
        assert_eq!(
            u64::MAX,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );

        // leverage out of limit
        position.price = scale(300_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            u64::MAX,
            pool.get_leverage(&position, &token_price, &token_price, &custody, 1, true)
                .unwrap()
        );
    }

    #[test]
    fn test_get_liquidation_price() {
        let (pool, custody, mut position, _) = get_fixture();

        assert_eq!(
            190776743335844,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );

        // lower price should lower liquidation price
        position.price = scale(24_500, Cortex::PRICE_DECIMALS);
        assert_eq!(
            186961208469127,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );

        position.price = scale(20_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            152621394668675,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );

        // higher price should increase liquidation price
        position.price = scale(26_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            198407813069278,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );

        position.price = scale(35_000, Cortex::PRICE_DECIMALS);
        assert_eq!(
            267087440670181,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );

        // dead price
        position.price = scale(0, Cortex::PRICE_DECIMALS);
        assert_eq!(
            0,
            pool.get_liquidation_price(&position, &custody, &custody, 1)
                .unwrap()
        );
    }

    #[test]
    fn test_get_exit_position_numbers() {
        let (pool, custody, position, token_price) = get_fixture();

        let amounts = pool
            .get_exit_position_numbers(&position, &token_price, &token_price, &custody, 1, false)
            .unwrap();

        assert_eq!(amounts.close_amount, scale_f64(0.996, custody.decimals));
        assert_eq!(amounts.exit_fee, 4000000);
        assert_eq!(amounts.exit_fee_usd, scale(100, Cortex::USD_DECIMALS));
        assert_eq!(amounts.borrow_fee, 0);
        assert_eq!(amounts.borrow_fee_usd, 0);
        assert_eq!(amounts.profit_usd, 0);
        assert_eq!(amounts.loss_usd, scale(100, Cortex::USD_DECIMALS));
    }

    #[test]
    fn test_get_unrealized_interest_amount_usd() {
        let (_pool, mut custody, mut position, _token_price) = get_fixture();

        custody.borrow_rate = BorrowRateParams {
            max_hourly_borrow_interest_rate: 100_000, // 0.01%
        };
        custody.assets.locked = scale(9, 9);
        custody.assets.owned = scale(10, 9);

        custody.update_borrow_rate(3_600).unwrap();
        let unrealized_interest = custody.get_interest_amount_usd(&position, 3_600).unwrap();
        assert_eq!(unrealized_interest, 0);

        let unrealized_interest = custody.get_interest_amount_usd(&position, 7_200).unwrap();
        assert_eq!(unrealized_interest, scale(9, Cortex::USD_DECIMALS));

        custody.update_borrow_rate(7_200).unwrap();
        let unrealized_interest = custody.get_interest_amount_usd(&position, 7_199).unwrap();
        assert_eq!(unrealized_interest, scale(9, Cortex::USD_DECIMALS));

        position.cumulative_interest_snapshot = U128Split::from(70_000);
        let unrealized_interest = custody.get_interest_amount_usd(&position, 7_200).unwrap();
        assert_eq!(unrealized_interest, scale(2, Cortex::USD_DECIMALS));
    }
}
