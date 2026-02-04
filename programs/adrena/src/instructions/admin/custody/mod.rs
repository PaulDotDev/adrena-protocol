pub mod add_custody;
pub mod patch_custodies_oracles;
pub mod patch_custody_locked_amount;
pub mod remove_custody;
pub mod set_custody_allow_swap;
pub mod set_custody_allow_trade;
pub mod set_custody_config;
pub mod set_custody_max_cumulative_short_position_size_usd;

pub use {
    add_custody::*, patch_custodies_oracles::*, patch_custody_locked_amount::*, remove_custody::*,
    set_custody_allow_swap::*, set_custody_allow_trade::*, set_custody_config::*,
    set_custody_max_cumulative_short_position_size_usd::*,
};
