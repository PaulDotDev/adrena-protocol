#![allow(non_snake_case)]

use {
    colored::*,
    ctor::dtor,
    once_cell::sync::Lazy,
    std::{
        collections::HashMap,
        fs::File,
        io::Write,
        sync::{Arc, Mutex},
    },
};
pub mod adapters;
pub mod test_instructions;
pub mod tests_suite;
pub mod utils;

/* @deprecated tests
* Patched - Kept for history but the IX is now commented
* tests_suite::genesis_liquidity::genesis_stake_patch_claim_all().await;
* tests_suite::genesis_liquidity::genesis_stake_patch_claim_none().await;
* tests_suite::genesis_liquidity::genesis_stake_patch_claim_middle().await;
* tests_suite::genesis_liquidity::genesis_stake_patch_not_concerned().await;
* tests_suite::staking::patch_staking_round().await;
// */

#[tokio::test]
pub async fn test_integration() {
    tests_suite::position::increase_btc_position().await;
    tests_suite::position::increase_bonk_position().await;

    tests_suite::basic_interactions().await;

    tests_suite::liquidity::fees().await;
    tests_suite::liquidity::insufficient_fund().await;

    tests_suite::position_with_swap::open_close_position_with_swap().await;

    tests_suite::position::add_collateral_long().await;
    tests_suite::position::increase_short_position().await;
    tests_suite::position::add_collateral_short().await;
    tests_suite::position::open_and_close_long_position_accounting().await;
    tests_suite::position::open_and_close_short_position_accounting().await;
    tests_suite::position::min_max_leverage().await;
    tests_suite::position::liquidate_position().await;
    tests_suite::position::max_user_profit().await;
    tests_suite::position::remove_collateral_long().await;
    tests_suite::position::remove_collateral_short().await;
    tests_suite::position::open_and_close_position_edge_case().await;
    tests_suite::position::max_cumulative_short().await;
    tests_suite::position::increase_position_locked_amount().await;
}

#[tokio::test]
pub async fn test_integration_part2() {
    tests_suite::staking::staking_rewards_generation().await;
    tests_suite::staking::liquid_staking().await;
    tests_suite::staking::locked_staking_180d_adx().await;
    tests_suite::staking::multiple_stakers_get_correct_rewards().await;
    tests_suite::staking::liquid_staking_overlap().await;
    tests_suite::staking::liquid_staking_overlap_remove_less_than_overlap().await;
    tests_suite::staking::liquid_staking_overlap_remove_same_as_overlap().await;
    tests_suite::staking::liquid_staking_overlap_remove_more_than_overlap().await;
    tests_suite::staking::liquid_staking_overlap_remove_less_than_overlap_2().await;
    tests_suite::staking::remove_locked_stake_early_adx().await;
    tests_suite::staking::lm_emission_potentiometer().await;
    tests_suite::staking::upgrade_locked_stake_amount_adx().await;
}

#[tokio::test]
pub async fn test_integration_part3() {
    tests_suite::lp_token::lp_token_price().await;

    tests_suite::pool::aum_soft_cap_usd().await;

    tests_suite::custody::max_cumulative_short_position_size_usd().await;

    // Views
    tests_suite::views::get_assets_under_management().await;
    tests_suite::views::get_liquidation_price().await;
    tests_suite::views::get_liquidation_state().await;

    // Long tests
    tests_suite::staking::resolved_round_overflow().await;
    tests_suite::staking::auto_claim().await;
    tests_suite::staking::auto_resolve().await;

    tests_suite::staking::claim_computing_limit().await;
    tests_suite::staking::test_aum_manipulation().await;
}

#[tokio::test]
pub async fn test_integration_part4() {
    tests_suite::vesting::vote().await;
    tests_suite::vesting::claim().await;
    tests_suite::vesting::vote_multiplier().await;
}

#[tokio::test]
pub async fn test_integration_part5() {
    tests_suite::get_assets_under_management_usd::single_long().await;
    tests_suite::get_assets_under_management_usd::single_short().await;
    tests_suite::position::open_and_close_bonk_position_x100().await;
    tests_suite::position::borrow_fee().await;
    tests_suite::position::resolve_position_borrow_fees().await;

    tests_suite::position::open_and_close_bonk_position().await;
    tests_suite::position::open_and_partial_close_bonk_position().await;
    tests_suite::position::open_and_partial_close_long_position_accounting().await;
    tests_suite::position::open_and_partial_close_short_position_accounting().await;
}

#[tokio::test]
pub async fn test_integration_part6() {
    tests_suite::swap::permissioned_swap().await;

    tests_suite::position::take_profit_long().await;
    tests_suite::position::stop_loss_long().await;
    tests_suite::position::take_profit_short().await;
    tests_suite::position::stop_loss_short().await;

    tests_suite::vesting::migrate_from_v1_to_v2().await;

    tests_suite::staking::upgrade_locked_stake_duration_adx().await;
    tests_suite::staking::upgrade_locked_stake_amount_and_duration_adx().await;
    tests_suite::staking::liquid_staking_overlap_party().await;
}

#[tokio::test]
pub async fn test_integration_part7() {
    tests_suite::user_profile::init_user_profile().await;
    tests_suite::user_profile::edit_user_profile().await;
    tests_suite::user_profile::migrate_user_profile_from_v1_to_v2().await;
    tests_suite::user_profile::edit_user_profile().await;
    tests_suite::user_profile::grant_or_remove_achievement_tests().await;
}

#[tokio::test]
pub async fn test_deep_checks() {
    tests_suite::swap::deep_check_swap().await;
    tests_suite::position::deep_check_open_position_long().await;
    tests_suite::position::deep_check_open_position_short().await;
    tests_suite::position::deep_check_add_collateral_long().await;
    tests_suite::position::deep_check_add_collateral_short().await;
    tests_suite::position::deep_check_remove_collateral_long().await;
    tests_suite::position::deep_check_remove_collateral_short().await;
    tests_suite::position::deep_check_close_position_long().await;
    tests_suite::position::deep_check_close_position_short().await;
    tests_suite::position::deep_check_liquidate_position_long().await;
    tests_suite::position::deep_check_liquidate_position_short().await;
}

#[tokio::test]
pub async fn test_integration_part8() {
    // tests_suite::many_prices_setup().await;
    tests_suite::lm_minting::mint_lm_tokens_from_bucket().await;
    tests_suite::lm_minting::mint_staked_lm_tokens_from_bucket().await;
    tests_suite::fees::referral_fee_regular_usecase().await;
    tests_suite::fees::fee_distribution_no_referrer().await;
}

#[tokio::test]
pub async fn test_offside_audit() {
    tests_suite::offside_audit::forged_realm().await;

    tests_suite::offside_audit::increase_position_long_issue_negative().await;
    tests_suite::offside_audit::increase_position_long_issue_positive().await;
    tests_suite::offside_audit::increase_position_long_issue_positive_bn().await;
    tests_suite::offside_audit::increase_position_long_issue_neutral().await;

    tests_suite::offside_audit::increase_position_short_issue_negative().await;
    tests_suite::offside_audit::increase_position_short_issue_positive().await;
    tests_suite::offside_audit::increase_position_short_issue_neutral().await;
}

//
// Globally store information about instruction sizes and compute units
// When tests finish, print the log of instructions with their sizes and compute units
//

#[derive(Debug, Default, Clone)]
pub struct InstructionInfo {
    name: String,
    category: String, // Add a category field
    max_size_bytes: usize,
    min_size_bytes: usize,
    max_compute_units: u64,
    min_compute_units: u64,
}

impl InstructionInfo {
    pub fn new(name: &str, category: &str, size_bytes: usize, compute_units: u64) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
            max_size_bytes: size_bytes,
            min_size_bytes: size_bytes,
            max_compute_units: compute_units,
            min_compute_units: compute_units,
        }
    }
}

pub static INSTRUCTION_LOG: Lazy<Arc<Mutex<Vec<InstructionInfo>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

// Function that runs when the process stops
#[dtor]
fn print_instruction_log() {
    const MAX_NAME_LENGTH: usize = 40; // Set the max display width for names
    let log = INSTRUCTION_LOG.lock().unwrap();
    let mut file = File::create("instructions_cu_size_log.txt").expect("Failed to create file");

    // Clone and group by category
    let mut grouped_by_category: HashMap<String, Vec<InstructionInfo>> = HashMap::new();
    for entry in log.iter() {
        grouped_by_category
            .entry(entry.category.clone())
            .or_default()
            .push(entry.clone());
    }

    // Iterate over each category
    for (category, mut instructions) in grouped_by_category {
        println!("\nCategory: {}", category.bold());
        writeln!(file, "\nCategory: {}", category).unwrap();

        // Sort each category by name in ascending order
        instructions.sort_by(|a, b| a.name.cmp(&b.name));

        // Print the header
        println!(
            "{:<42} {:<29} {:<28}",
            "Instruction".bold(),
            "Compute Units".bold(),
            "TX Byte Size".bold()
        );
        writeln!(
            file,
            "{:<42} {:<29} {:<28}",
            "Instruction", "Compute Units", "TX Byte Size"
        )
        .unwrap();
        println!(
            "{:<40} {:<14} {:<14} {:<13} {:<13}",
            "",
            "Max".bold(),
            "Min".bold(),
            "Max".bold(),
            "Min".bold()
        );
        writeln!(
            file,
            "{:<40} {:<14} {:<14} {:<13} {:<13}",
            "", "Max", "Min", "Max", "Min"
        )
        .unwrap();
        writeln!(file, "{}", "-".repeat(84)).unwrap();

        // Print each instruction
        for info in instructions.iter() {
            let truncated_name = if info.name.len() > MAX_NAME_LENGTH {
                format!("{}...", &info.name[..MAX_NAME_LENGTH - 3])
            } else {
                info.name.clone()
            };

            // Apply colors based on thresholds
            let max_cu_color = if info.max_compute_units > 1_000_000 {
                info.max_compute_units.to_string().red()
            } else if info.max_compute_units > 300_000 {
                info.max_compute_units.to_string().yellow()
            } else {
                info.max_compute_units.to_string().green()
            };

            let min_cu_color = if info.min_compute_units > 1_000_000 {
                info.min_compute_units.to_string().red()
            } else if info.min_compute_units > 300_000 {
                info.min_compute_units.to_string().yellow()
            } else {
                info.min_compute_units.to_string().green()
            };

            let max_size_color = if info.max_size_bytes > 1000 {
                info.max_size_bytes.to_string().red()
            } else if info.max_size_bytes > 600 {
                info.max_size_bytes.to_string().yellow()
            } else {
                info.max_size_bytes.to_string().green()
            };

            let min_size_color = if info.min_size_bytes > 1000 {
                info.min_size_bytes.to_string().red()
            } else if info.min_size_bytes > 600 {
                info.min_size_bytes.to_string().yellow()
            } else {
                info.min_size_bytes.to_string().green()
            };

            println!(
                "{:<40} {:<14} {:<14} {:<13} {:<13}",
                truncated_name, max_cu_color, min_cu_color, max_size_color, min_size_color
            );
            writeln!(
                file,
                "{:<40} {:<14} {:<14} {:<13} {:<13}",
                truncated_name,
                info.max_compute_units,
                info.min_compute_units,
                info.max_size_bytes,
                info.min_size_bytes
            )
            .unwrap();
        }
    }
}
