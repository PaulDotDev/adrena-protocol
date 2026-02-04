use anchor_lang::prelude::*;

#[event]
pub struct OpenPositionEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub size_usd: u64,
    pub price: u64,
    pub collateral_amount_usd: u64,
    pub leverage: u32,
    pub position_id: u64,
}

#[event]
pub struct IncreasePositionEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub size_usd: u64,
    pub price: u64,
    pub collateral_amount_usd: u64,
    pub leverage: u32,
    pub position_id: u64,
}

#[event]
pub struct ClosePositionEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub size_usd: u64,
    pub price: u64,
    pub collateral_amount_usd: u64,
    pub profit_usd: u64,
    pub loss_usd: u64,
    pub borrow_fee_usd: u64,
    pub exit_fee_usd: u64,
    pub position_id: u64,
    pub percentage: u64,
}

#[event]
pub struct AddCollateralEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub add_amount_usd: u64,
    pub new_collateral_amount_usd: u64,
    pub leverage: u32,
    pub position_id: u64,
}

#[event]
pub struct RemoveCollateralEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub remove_amount_usd: u64,
    pub new_collateral_amount_usd: u64,
    pub leverage: u32,
    pub position_id: u64,
}

#[event]
pub struct LiquidateEvent {
    pub owner: Pubkey,
    pub position: Pubkey,
    pub custody_mint: Pubkey,
    pub side: u8, // Side
    pub size_usd: u64,
    pub price: u64,
    pub collateral_amount_usd: u64,
    pub loss_usd: u64,
    pub borrow_fee_usd: u64,
    pub exit_fee_usd: u64,
    pub position_id: u64,
}

#[event]
pub struct AddLockedStakeEvent {
    pub owner: Pubkey,
    pub staking: Pubkey,
    pub locked_stake_id: u64,
    pub amount: u64,
    pub locked_days: u32,
}

#[event]
pub struct UpgradeLockedStakeEvent {
    pub owner: Pubkey,
    pub staking: Pubkey,
    pub locked_stake_id: u64,
    pub amount: Option<u64>,
    pub locked_days: Option<u32>,
}

#[event]
pub struct FinalizeLockedStakeEvent {
    pub owner: Pubkey,
    pub staking: Pubkey,
    pub locked_stake_id: u64,
    pub early_exit: bool,
}

#[event]
pub struct RemoveLockedStakeEvent {
    pub owner: Pubkey,
    pub staking: Pubkey,
    pub locked_stake_id: u64,
}

#[event]
pub struct SetStopLossEvent {
    pub position_id: u64,
    pub stop_loss_limit_price: u64,
    pub close_position_price: Option<u64>,
    pub position_side: u8,
}

#[event]
pub struct SetTakeProfitEvent {
    pub position_id: u64,
    pub take_profit_limit_price: u64,
    pub position_side: u8,
}

#[event]
pub struct CancelStopLossEvent {
    pub position_id: u64,
    pub position_side: u8,
}

#[event]
pub struct CancelTakeProfitEvent {
    pub position_id: u64,
    pub position_side: u8,
}
