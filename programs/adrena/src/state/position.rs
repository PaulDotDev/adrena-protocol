use {
    super::cortex::Cortex,
    crate::{error::AdrenaError, math, utils::u128_split::U128Split},
    anchor_lang::prelude::*,
};

// A position cannot be opened and closed in less than 20 seconds
pub const MIN_POSITION_OPEN_TIME_SECONDS: u8 = 20;

#[derive(PartialEq, Copy, Clone, Default, Debug)]
pub enum Side {
    None = 0,
    #[default]
    Long = 1,
    Short = 2,
}

impl From<Side> for u8 {
    fn from(val: Side) -> Self {
        match val {
            Side::None => 0,
            Side::Long => 1,
            Side::Short => 2,
        }
    }
}

impl TryFrom<u8> for Side {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => Side::None,
            1 => Side::Long,
            2 => Side::Short,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidPositionState)?,
        })
    }
}

#[account(zero_copy)]
#[derive(Default, Debug)]
#[repr(C)]
pub struct Position {
    pub bump: u8,
    pub side: u8, // Side
    pub take_profit_is_set: u8,
    pub stop_loss_is_set: u8,
    // This was used before for the sablier integration, if reusing, make sure to init the content
    pub _padding_unsafe: [u8; 1],
    pub _padding: [u8; 3],

    pub owner: Pubkey,
    pub pool: Pubkey,
    pub custody: Pubkey,
    pub collateral_custody: Pubkey,

    pub open_time: i64,
    pub update_time: i64,
    pub price: u64,
    pub size_usd: u64,
    pub borrow_size_usd: u64,
    pub collateral_usd: u64,
    // Borrowing fee to be paid (increasing after increase position)
    pub unrealized_interest_usd: u64,
    pub cumulative_interest_snapshot: U128Split,
    pub locked_amount: u64,
    pub collateral_amount: u64,

    // Estimations at open - When actually closing, the confidence will be applied
    // Before being used again at close time, these values are updated to their true values
    pub exit_fee_usd: u64,
    pub liquidation_fee_usd: u64,

    pub id: u64,
    pub take_profit_limit_price: u64,
    pub paid_interest_usd: u64,
    pub stop_loss_limit_price: u64,
    // 0 means no slippage
    pub stop_loss_close_position_price: u64,
}

// Utility function to downscale a field by a percentage
fn apply_downscale(
    field: &mut u64,
    percentage: u64,
) -> std::result::Result<(), anchor_lang::error::Error> {
    let scaled =
        math::checked_as_u64(*field as u128 * percentage as u128 / (Cortex::BPS_POWER * 100))?;

    *field = field.checked_sub(scaled).ok_or(AdrenaError::MathOverflow)?;

    Ok(())
}

impl Position {
    pub const LEN: usize = 8 + std::mem::size_of::<Position>();

    // Downscale the position by an X factor
    pub fn downscale(
        &mut self,
        percentage: u64,
    ) -> std::result::Result<(), anchor_lang::error::Error> {
        apply_downscale(&mut self.size_usd, percentage)?;
        apply_downscale(&mut self.borrow_size_usd, percentage)?;
        apply_downscale(&mut self.collateral_usd, percentage)?;
        apply_downscale(&mut self.unrealized_interest_usd, percentage)?;
        apply_downscale(&mut self.locked_amount, percentage)?;
        apply_downscale(&mut self.collateral_amount, percentage)?;
        apply_downscale(&mut self.exit_fee_usd, percentage)?;
        apply_downscale(&mut self.liquidation_fee_usd, percentage)?;
        apply_downscale(&mut self.paid_interest_usd, percentage)?;

        Ok(())
    }

    pub fn subtract(
        &mut self,
        position: &Position,
    ) -> std::result::Result<(), anchor_lang::error::Error> {
        self.size_usd -= position.size_usd;
        self.borrow_size_usd -= position.borrow_size_usd;
        self.collateral_usd -= position.collateral_usd;
        self.unrealized_interest_usd -= position.unrealized_interest_usd;
        self.locked_amount -= position.locked_amount;
        self.collateral_amount -= position.collateral_amount;
        self.exit_fee_usd -= position.exit_fee_usd;
        self.liquidation_fee_usd -= position.liquidation_fee_usd;
        self.paid_interest_usd -= position.paid_interest_usd;

        Ok(())
    }

    pub fn get_side(&self) -> Side {
        // Consider value in the struct always good
        Side::try_from(self.side).unwrap()
    }

    pub fn take_profit_is_set(&self) -> bool {
        self.take_profit_is_set != 0
    }

    pub fn stop_loss_is_set(&self) -> bool {
        self.stop_loss_is_set != 0
    }

    pub fn take_profit_reached(&self, price: u64) -> bool {
        if self.take_profit_limit_price == 0 {
            return false;
        }

        if self.get_side() == Side::Long {
            price >= self.take_profit_limit_price
        } else {
            price <= self.take_profit_limit_price
        }
    }

    pub fn stop_loss_reached(&self, price: u64) -> bool {
        if self.stop_loss_limit_price == 0 {
            return false;
        }

        if self.get_side() == Side::Long {
            price <= self.stop_loss_limit_price
        } else {
            price >= self.stop_loss_limit_price
        }
    }

    pub fn stop_loss_slippage_ok(&self, price: u64) -> bool {
        // 0 means no slippage
        if self.stop_loss_close_position_price == 0 {
            return true;
        }

        if self.get_side() == Side::Long {
            price >= self.stop_loss_close_position_price
        } else {
            price <= self.stop_loss_close_position_price
        }
    }
}
