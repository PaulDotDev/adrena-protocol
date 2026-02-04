use {
    super::{
        cortex::{Cortex, HOURS_PER_DAY, SECONDS_PER_HOURS},
        staking::{StakingRound, StakingType},
    },
    crate::{error::AdrenaError, math},
    anchor_lang::prelude::*,
    bytemuck::{Pod, Zeroable},
};

pub const MAX_LOCKED_STAKE_COUNT: usize = 32;

#[account(zero_copy)]
#[derive(Default, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct UserStaking {
    pub bump: u8,
    pub _unused_unsafe: [u8; 1],
    pub staking_type: u8,
    pub _padding: [u8; 5],
    pub locked_stake_id_counter: u64,
    pub liquid_stake: LiquidStake,
    pub locked_stakes: [LockedStake; MAX_LOCKED_STAKE_COUNT],
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Zeroable, Pod,
)]
#[repr(C)]
pub struct LiquidStake {
    pub amount: u64,
    pub stake_time: i64,
    //
    // Time used for claim purpose, to know wherever the stake is eligible for round reward
    pub claim_time: i64,
    //
    // When user add stake when a stake is already live
    pub overlap_time: i64,
    pub overlap_amount: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Zeroable, Pod,
)]
#[repr(C)]
pub struct LockedStake {
    pub amount: u64,
    pub stake_time: i64,
    //
    // Last time tokens have been claimed for this stake
    pub claim_time: i64,
    // Time at which the locked stake will end (no more reward accrual) - Set at open depending of duration
    // and updated if early unstake
    pub end_time: i64,
    //
    // In seconds
    pub lock_duration: u64,
    //
    // In BPS
    pub reward_multiplier: u32,
    pub lm_reward_multiplier: u32,
    pub vote_multiplier: u32,
    //
    // This value is used to know if the take is eligible for Early unstake and Upgrade locked stake (in both case there are overlap issue,
    // could be managed as it's done with overlap in liquid stake but we want to avoid resizing account and there isn't enough space for this as of now 11/04/2024)
    pub qualified_for_rewards_in_resolved_round_count: u32,
    //
    // Persisted data to save-up computation during claim etc.
    // amount with base reward multiplier applied to it
    pub amount_with_reward_multiplier: u64,
    // amount with base reward multiplier applied to it
    pub amount_with_lm_reward_multiplier: u64,
    //
    // locked stake needs to be resolved before removing it
    // doesn't apply to liquid stake (lock_duration == 0)
    pub resolved: u8,
    pub _padding2: [u8; 7],
    // History: was a thread id before while using Sablier, now used as a unique random id for each stake
    pub id: u64,
    //
    pub early_exit: u8,
    pub _padding3: [u8; 7],
    // Calculated when finalizing the locked stake exiting early
    pub early_exit_fee: u64,
    //
    // GENESIS LOCK Specific
    pub is_genesis: u8,
    pub _padding4: [u8; 7],
    // Last time the genesis rewards has been claimed for this stake
    pub genesis_claim_time: i64,
}

impl LiquidStake {
    // This only makes sense on rounds that are already passed. Do not use it for current rounds.
    pub fn qualifies_for_rewards_from(&self, staking_round: &StakingRound) -> bool {
        self.stake_time > 0
            && self.stake_time < staking_round.start_time
            && (self.claim_time == 0 || self.claim_time < staking_round.start_time)
    }
}

impl LockedStake {
    pub const FEE_RATE_UPPER_CAP: u128 = 400_000_000; // 40%
    pub const FEE_RATE_LOWER_CAP: u128 = 50_000_000; // 5%

    pub fn is_initialized(&self) -> bool {
        self.amount > 0
    }

    pub fn is_genesis(&self) -> bool {
        self.is_genesis != 0
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved != 0
    }

    pub fn is_early_exit(&self) -> bool {
        self.early_exit != 0
    }

    // See comment on self.qualified_for_rewards_in_resolved_rounds_count
    pub fn is_established(&self) -> bool {
        self.qualified_for_rewards_in_resolved_round_count >= 1
    }

    pub fn qualifies_for_rewards_from(&self, staking_round: &StakingRound) -> bool {
        self.stake_time > 0
            && self.stake_time < staking_round.start_time
            && (self.claim_time == 0 || self.claim_time < staking_round.start_time)
            && staking_round.end_time <= self.end_time
            && staking_round.start_time < self.end_time
    }

    pub fn has_ended(&self, current_time: i64) -> Result<bool> {
        require!(self.stake_time > 0, AdrenaError::InvalidStakeState);
        require!(self.is_initialized(), AdrenaError::InvalidStakeState);

        Ok(self.end_time <= current_time)
    }

    // Returns the amount of the tax that will be applied to the early unstake,
    // pro rata of the lock period and lock time left.
    // The fee is capped between lower and upper caps
    // Expressed in the staked token denomination (native units)
    pub fn calculate_early_exit_fee_amount(&self, current_time: i64) -> Result<u64> {
        let time_elapsed = current_time - self.stake_time;
        let time_remaining = self.lock_duration as i64 - time_elapsed;

        // Calculate the fee rate as a percentage of the lock duration remaining
        let fee_rate = time_remaining as u128 * Cortex::RATE_POWER / self.lock_duration as u128;

        // Cap the fee rate between the lower and upper caps
        let capped_fee_rate = std::cmp::min(
            std::cmp::max(fee_rate, Self::FEE_RATE_LOWER_CAP),
            Self::FEE_RATE_UPPER_CAP,
        );

        // Calculate the fee amount as a percentage of the token unstake amount
        // Here, capped_fee_rate is already a percentage (e.g., 15% to 50% of RATE_POWER),
        // so we divide by RATE_POWER to apply it to the token_unstake_amount.
        let early_exit_fee_amount =
            math::checked_as_u64(self.amount as u128 * capped_fee_rate / Cortex::RATE_POWER)?;

        Ok(early_exit_fee_amount)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct LockedStakingOption {
    pub locked_days: u32,
    pub reward_multiplier: u32,
    pub lm_reward_multiplier: u32,
    pub vote_multiplier: u32,
}

impl LockedStakingOption {
    pub fn calculate_end_of_staking(&self, start: i64) -> Result<i64> {
        Ok(start + (SECONDS_PER_HOURS * HOURS_PER_DAY * self.locked_days as i64))
    }
}

// List of valid locked staking options and the related multipliers
pub const LOCKED_LM_STAKING_OPTIONS: [&LockedStakingOption; 5] = [
    &LockedStakingOption {
        locked_days: 0,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 1.0) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 0.0) as u32,
        vote_multiplier: (Cortex::BPS_POWER as f64 * 1.0) as u32,
    },
    &LockedStakingOption {
        locked_days: 90,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 1.75) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 1.0) as u32,
        vote_multiplier: (Cortex::BPS_POWER as f64 * 1.75) as u32,
    },
    &LockedStakingOption {
        locked_days: 180,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 2.5) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 1.75) as u32,
        vote_multiplier: (Cortex::BPS_POWER as f64 * 2.5) as u32,
    },
    &LockedStakingOption {
        locked_days: 360,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 3.25) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 2.5) as u32,
        vote_multiplier: (Cortex::BPS_POWER as f64 * 3.25) as u32,
    },
    &LockedStakingOption {
        locked_days: 540,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 4.0) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 3.25) as u32,
        vote_multiplier: (Cortex::BPS_POWER as f64 * 4.0) as u32,
    },
];

pub const LOCKED_LP_STAKING_OPTIONS: [&LockedStakingOption; 5] = [
    // THIS IS A NON OPTION - here for parity of stake duration between LM and LP.
    &LockedStakingOption {
        locked_days: 0,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 0.0) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 0.0) as u32,
        vote_multiplier: 0,
    },
    &LockedStakingOption {
        locked_days: 90,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 0.75) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 1.0) as u32,
        vote_multiplier: 0,
    },
    &LockedStakingOption {
        locked_days: 180,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 1.5) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 1.75) as u32,
        vote_multiplier: 0,
    },
    &LockedStakingOption {
        locked_days: 360,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 2.25) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 2.5) as u32,
        vote_multiplier: 0,
    },
    &LockedStakingOption {
        locked_days: 540,
        reward_multiplier: (Cortex::BPS_POWER as f64 * 3.0) as u32,
        lm_reward_multiplier: (Cortex::BPS_POWER as f64 * 3.25) as u32,
        vote_multiplier: 0,
    },
];

impl UserStaking {
    pub const LEN: usize = 8 + std::mem::size_of::<UserStaking>();

    // The max age of a UserStaking account in the system, 9 days
    pub const MAX_AGE_SECONDS: i64 = 8 * HOURS_PER_DAY * SECONDS_PER_HOURS;

    // The max amount of locked stakes a user can have simultaneously
    pub const MAX_LOCKED_STAKES: usize = MAX_LOCKED_STAKE_COUNT;

    pub fn get_next_locked_stake_id(&mut self) -> u64 {
        let id = self.locked_stake_id_counter;
        // Check if id already exists in locked stakes
        self.locked_stake_id_counter = self.locked_stake_id_counter.wrapping_add(1);
        if self
            .locked_stakes
            .iter()
            .any(|stake| stake.is_initialized() && stake.id == id)
        {
            self.get_next_locked_stake_id() // Try next id if current exists
        } else {
            id
        }
    }

    pub fn locked_stakes_is_empty(&self) -> bool {
        self.locked_stakes
            .iter()
            .filter(|stake| stake.is_initialized())
            .count()
            == 0
    }

    pub fn add_locked_stake(&mut self, new_stake: LockedStake) -> Result<()> {
        let mut added = false;
        for stake in self.locked_stakes.iter_mut() {
            // if the spot is uninitialized, it's empty and used
            if !stake.is_initialized() {
                *stake = new_stake;
                added = true;
                break;
            }
        }

        if !added {
            return Err(AdrenaError::LockedStakeArrayFull.into());
        }

        Ok(())
    }

    pub fn remove_locked_stake(&mut self, index: usize) -> Result<()> {
        if index >= self.locked_stakes.len() {
            return Err(AdrenaError::IndexOutOfBounds.into());
        }

        let locked_stake = &mut self.locked_stakes[index];

        // Reset the locked stake at the specified index
        *locked_stake = LockedStake::default();

        Ok(())
    }

    pub fn get_locked_staking_option(
        locked_days: u32,
        staking_type: StakingType,
    ) -> Result<LockedStakingOption> {
        let options = if staking_type == StakingType::LM {
            LOCKED_LM_STAKING_OPTIONS
        } else {
            LOCKED_LP_STAKING_OPTIONS
        };

        let staking_option = options
            .into_iter()
            .find(|period| period.locked_days == locked_days);

        require!(
            staking_option.is_some(),
            AdrenaError::InvalidStakingLockingTime
        );

        // Prevent 0d locked staking for LM
        if staking_type == StakingType::LM && locked_days == 0 {
            return Err(AdrenaError::InvalidStakingLockingTime.into());
        }

        Ok(*staking_option.unwrap())
    }

    pub fn get_staking_type(&self) -> StakingType {
        // Consider the value inside the struct always good
        StakingType::try_from(self.staking_type).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_early_unstake_fee_rate() {
        // Setup
        let locked_stake = LockedStake {
            amount: 1000,
            stake_time: 100,                     // Example stake time
            lock_duration: 200,                  // Example lock duration (e.g., 200 seconds)
            claim_time: 0,                       // Not relevant for this test
            reward_multiplier: 0,                // Not relevant for this test
            lm_reward_multiplier: 0,             // Not relevant for this test
            vote_multiplier: 0,                  // Not relevant for this test
            amount_with_reward_multiplier: 0,    // Not relevant for this test
            amount_with_lm_reward_multiplier: 0, // Not relevant for this test
            resolved: false as u8,               // Not relevant for this test
            id: 0,                               // Not relevant for this test
            genesis_claim_time: 0,               // Not relevant for this test
            is_genesis: false as u8,             // Not relevant for this test
            early_exit: false as u8,             // Not relevant for this test
            early_exit_fee: 0,                   // Not relevant for this test
            ..LockedStake::default()
        };

        // 1 - direct unstake, should be taxed at max value
        let current_time = 101;

        // Execution
        let early_exit_fee_amount = locked_stake
            .calculate_early_exit_fee_amount(current_time)
            .unwrap();

        // Assertion
        let expected_amount =
            math::checked_as_u64(1000 * LockedStake::FEE_RATE_UPPER_CAP / Cortex::RATE_POWER)
                .unwrap(); // 400
        assert_eq!(
            early_exit_fee_amount, expected_amount,
            "The fee rate calculated did not match the expected value."
        );

        // 2 - unstake near the end, should be taxed at min value
        let current_time = 299;

        // Execution
        let early_exit_fee_amount = locked_stake
            .calculate_early_exit_fee_amount(current_time)
            .unwrap();

        // Assertion
        let expected_amount =
            math::checked_as_u64(1000 * LockedStake::FEE_RATE_LOWER_CAP / Cortex::RATE_POWER)
                .unwrap(); // 150
        assert_eq!(
            early_exit_fee_amount, expected_amount,
            "The fee rate calculated did not match the expected value."
        );

        // 3 - 75% unstake, should be taxed at proportional value
        let current_time = 250;

        // Execution
        let early_exit_fee_amount = locked_stake
            .calculate_early_exit_fee_amount(current_time)
            .unwrap();

        // Assertion
        let expected_amount = 250;
        assert_eq!(
            early_exit_fee_amount, expected_amount,
            "The fee rate calculated did not match the expected value."
        );
    }

    #[test]
    fn test_get_next_locked_stake_id() {
        let mut user_staking = UserStaking {
            locked_stake_id_counter: 0,
            locked_stakes: [LockedStake::default(); MAX_LOCKED_STAKE_COUNT],
            ..UserStaking::default()
        };

        // Add a locked stake with a specific ID
        user_staking.locked_stakes[0] = LockedStake {
            id: 0,
            amount: 100,
            ..LockedStake::default()
        };

        // Add a locked stake with a specific ID
        user_staking.locked_stakes[1] = LockedStake {
            id: 1,
            amount: 100,
            ..LockedStake::default()
        };

        // Get the next locked stake ID
        let next_id = user_staking.get_next_locked_stake_id();

        // Assert that the next ID is not the same as the existing one
        assert_ne!(next_id, 0, "The next locked stake ID should not be 0");

        // Assert that the next ID is 2, as 0 and 1 are already taken
        assert_eq!(next_id, 2, "The next locked stake ID should be 2");
    }
}
