use {
    super::cortex::{Cortex, SECONDS_PER_HOURS},
    crate::{error::AdrenaError, math},
    anchor_lang::prelude::*,
    bytemuck::{Pod, Zeroable},
};

// a staking round can be resolved after at least 6 hours
pub const ROUND_MIN_DURATION_HOURS: i64 = 6;

pub const ROUND_MIN_DURATION_SECONDS: i64 = ROUND_MIN_DURATION_HOURS * SECONDS_PER_HOURS;

// A UserStaking account max age is 365, this is due to computing limit in the claim instruction.
// This is also arbitrarily used as the max theoretical amount of staking rounds
// stored if all were persisting (rounds get cleaned up once their rewards are fully claimed by their participants).
// This is done to ensure the resolved_staking_rounds doesn't grow out of proportion, primarily to facilitate
// the fetching from front end.
//
// Calculation: (UserStaking::MAX_AGE_SECONDS / SECONDS_PER_HOURS) / ROUND_MIN_DURATION_HOURS
pub const MAX_RESOLVED_ROUNDS: usize = 32;

// All calculations based on an average 30 days month
pub const SECONDS_PER_MONTH: i64 = 30 * SECONDS_PER_HOURS * 24;

// Theoretical max rounds per month, based on the min duration of a round of 6h
pub const MAX_ROUNDS_PER_MONTH: u64 = SECONDS_PER_MONTH as u64 / ROUND_MIN_DURATION_SECONDS as u64;

#[derive(PartialEq, Copy, Clone, Debug, Default)]
pub enum StakingType {
    #[default]
    LM = 1,
    LP = 2,
}

impl From<StakingType> for u8 {
    fn from(val: StakingType) -> Self {
        match val {
            StakingType::LM => 1,
            StakingType::LP => 2,
        }
    }
}

impl TryFrom<u8> for StakingType {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => StakingType::LM,
            2 => StakingType::LP,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidStakingState)?,
        })
    }
}

#[derive(PartialEq, Copy, Clone, Default, Debug)]
pub enum StakingInitializationStep {
    #[default]
    NotCreated = 0,
    Step1 = 1,
    Step2 = 2,
    Step3 = 3,
    Initialized = 4,
}

impl From<StakingInitializationStep> for u8 {
    fn from(val: StakingInitializationStep) -> Self {
        match val {
            StakingInitializationStep::NotCreated => 0,
            StakingInitializationStep::Step1 => 1,
            StakingInitializationStep::Step2 => 2,
            StakingInitializationStep::Step3 => 3,
            StakingInitializationStep::Initialized => 4,
        }
    }
}

impl TryFrom<u8> for StakingInitializationStep {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => StakingInitializationStep::NotCreated,
            1 => StakingInitializationStep::Step1,
            2 => StakingInitializationStep::Step2,
            3 => StakingInitializationStep::Step3,
            4 => StakingInitializationStep::Initialized,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidStakingState)?,
        })
    }
}

#[account(zero_copy)]
#[derive(Default, Debug)]
#[repr(C)]
pub struct Staking {
    pub staking_type: u8, // StakingType
    //
    // Bumps
    //
    pub bump: u8,
    pub staked_token_vault_bump: u8,
    pub reward_token_vault_bump: u8,
    pub lm_reward_token_vault_bump: u8,
    //
    pub reward_token_decimals: u8,
    pub staked_token_decimals: u8,
    //
    pub initialized: u8, // StakingInitializationStep
    //
    // Tokens in stake
    //
    pub nb_locked_tokens: u64,
    pub nb_liquid_tokens: u64,
    //
    // Token to stake
    //
    pub staked_token_mint: Pubkey,
    //
    // Resolved amounts
    //
    // amount of rewards allocated to resolved rounds, claimable (excluding current/next round)
    pub resolved_reward_token_amount: u64,
    // amount of staked token locked in resolved rounds
    pub resolved_staked_token_amount: u64,
    // amount of lm rewards allocated to resolved rounds, claimable (excluding current/next round)
    pub resolved_lm_reward_token_amount: u64,
    // amount of lm staked token locked in resolved rounds
    pub resolved_lm_staked_token_amount: u64,
    //
    // Staking rounds
    //
    pub current_staking_round: StakingRound,
    #[deprecated]
    pub current_staking_round_liquid_rewards_usd: u64,
    pub _padding1: [u8; 16],
    pub next_staking_round: NextStakingRound,
    pub _padding2: [u8; 8],
    pub resolved_staking_rounds: [StakingRound; MAX_RESOLVED_ROUNDS],
    pub registered_resolved_staking_round_count: u8,
    pub _padding3: [u8; 3],
    //
    // Token LM emission potentiometer
    // baseline is 10_000 (100%)
    pub lm_emission_potentiometer_bps: u16,
    pub months_elapsed_since_inception: u16,
    pub _padding_unsafe: [u8; 8],
    // Date at with the `current_month_emission_amount_per_round` was calculated last
    pub emission_amount_per_round_last_update: i64,
    // Amount of rewards to be distributed per staking round
    pub current_month_emission_amount_per_round: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct StakingRound {
    pub start_time: i64,
    // Unknown until ended (min value being ROUND_MIN_DURATION_SECONDS from start_time)
    pub end_time: i64,
    //
    // The amount of reward you get per staked stake-token for that round - set at Round's resolution
    pub rate: u64,
    // Set at Round's resolution
    pub total_stake: u64,
    // Set at Round's resolution
    pub total_claim: u64,
    //
    // The amount of lm reward you get per staked stake-token for that round - set at Round's resolution
    pub lm_rate: u64,
    // Set at Round's resolution
    pub lm_total_stake: u64,
    // Set at Round's resolution
    pub lm_total_claim: u64,
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct NextStakingRound {
    // Set at Round's resolution
    pub total_stake: u64,
    pub _padding1: [u8; 16],
    // Set at Round's resolution
    pub lm_total_stake: u64,
}

impl StakingRound {
    pub const LEN: usize = std::mem::size_of::<StakingRound>();

    pub const MAX_RESOLVED_ROUNDS: usize = MAX_RESOLVED_ROUNDS;
    pub const MAX_ROUNDS_PER_MONTH: u64 = MAX_ROUNDS_PER_MONTH;
    pub const ROUND_MIN_DURATION_SECONDS: i64 = ROUND_MIN_DURATION_SECONDS;

    pub fn new(start_time: i64) -> Self {
        Self {
            start_time,
            end_time: 0,
            rate: u64::MIN,
            total_stake: u64::MIN,
            total_claim: u64::MIN,
            lm_rate: u64::MIN,
            lm_total_stake: u64::MIN,
            lm_total_claim: u64::MIN,
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.start_time > 0
    }
}

impl Staking {
    // 8 bytes for anchor discriminator
    pub const LEN: usize = 8 + std::mem::size_of::<Staking>();
    pub const LM_EMISSION_POTENTIOMETER_BASELINE_BPS: u16 = 10_000; // x1
    pub const LM_EMISSION_POTENTIOMETER_MIN_BPS: u16 = 0; // x0
    pub const LM_EMISSION_POTENTIOMETER_MAX_BPS: u16 = 20_000; // x2

    pub const LP_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE: u64 = 100_000_000; // (10%)
    pub const LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y1: u64 = 70_000_000; // (7%)
    pub const LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y2: u64 = 120_000_000; // (12%)

    pub const LM_STAKING_REWARDS_EMISSION_FIRST_MONTH_AMOUNT: u64 =
        8_600_000 * 10u64.pow(Cortex::LM_DECIMALS as u32);
    pub const LP_STAKING_REWARDS_EMISSION_FIRST_MONTH_AMOUNT: u64 =
        15_000_000 * 10u64.pow(Cortex::LM_DECIMALS as u32);

    pub fn current_staking_round_is_resolvable(&self, current_time: i64) -> Result<bool> {
        Ok(current_time >= self.current_staking_round.start_time + ROUND_MIN_DURATION_SECONDS)
    }

    pub fn find_oldest_resolved_staking_round(&self) -> Option<usize> {
        let mut oldest_index: Option<usize> = None;
        let mut oldest_time: i64 = i64::MAX;

        for (index, round) in self.resolved_staking_rounds.iter().enumerate() {
            if round.is_resolved() && round.start_time < oldest_time {
                oldest_time = round.start_time;
                oldest_index = Some(index);
            }
        }

        oldest_index
    }

    pub fn get_staking_type(&self) -> StakingType {
        // Consider the value inside the struct always good
        StakingType::try_from(self.staking_type).unwrap()
    }

    pub fn get_initialized(&self) -> StakingInitializationStep {
        // Consider the value inside the struct always good
        StakingInitializationStep::try_from(self.initialized).unwrap()
    }

    pub fn is_initialized(&self) -> bool {
        self.get_initialized() == StakingInitializationStep::Initialized
    }

    pub fn remove_resolved_staking_round(&mut self, index: usize) {
        self.resolved_staking_rounds[index] = StakingRound::default();

        self.registered_resolved_staking_round_count -= 1;
    }

    pub fn add_resolved_staking_round(
        &mut self,
        resolved_staking_round: StakingRound,
    ) -> Result<()> {
        let spot = self
            .resolved_staking_rounds
            .iter()
            .position(|p| *p == StakingRound::default());

        match spot {
            Some(index) => {
                self.resolved_staking_rounds[index] = resolved_staking_round;
                self.registered_resolved_staking_round_count += 1;
                Ok(())
            }
            None => Err(AdrenaError::MaxRegisteredResolvedStakingRoundReached.into()),
        }
    }

    pub fn apply_emission_filter(&self, initial_amount: u64) -> Result<u64> {
        // Calculate the new amount with proper rounding
        let multiplier_bps = self.lm_emission_potentiometer_bps as u128;

        let new_amount =
            math::checked_as_u64((initial_amount as u128 * multiplier_bps) / Cortex::BPS_POWER)?;

        Ok(new_amount)
    }

    // must be between 0.00% and 200%
    pub fn validate_lm_emission_potentiometer_bps_in_range(&self, bps: u16) -> bool {
        (Self::LM_EMISSION_POTENTIOMETER_MIN_BPS..=Self::LM_EMISSION_POTENTIOMETER_MAX_BPS)
            .contains(&bps)
    }

    // LM STAKING - Return the amount of rewards to be distributed for the round based on the emission schedule starting value and decay per month
    pub fn update_lm_emission_amount_per_round_for_lm_staking_if_needed(
        &mut self,
        current_time: i64,
    ) -> Result<()> {
        // Has one month passed since the last update?
        if current_time < self.emission_amount_per_round_last_update + SECONDS_PER_MONTH {
            return Ok(());
        }

        // Is there update left to distribute? Should never happen given the % based decay rate
        if self.current_month_emission_amount_per_round == 0 {
            return Ok(());
        }

        let decay_rate = if self.months_elapsed_since_inception < 12 {
            Self::LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y1
        } else {
            Self::LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y2
        };

        let decay_amount = math::checked_as_u64(
            (self.current_month_emission_amount_per_round as u128 * decay_rate as u128)
                / Cortex::RATE_POWER,
        )?;

        self.current_month_emission_amount_per_round = self
            .current_month_emission_amount_per_round
            .saturating_sub(decay_amount);

        self.emission_amount_per_round_last_update = current_time;

        self.months_elapsed_since_inception += 1;

        Ok(())
    }

    // LP STAKING - Return the amount of rewards to be distributed for the round based on the emission schedule starting value and decay per month
    pub fn update_lm_emission_amount_per_round_for_lp_staking_if_needed(
        &mut self,
        current_time: i64,
    ) -> Result<()> {
        // Has one month passed since the last update?
        if current_time < self.emission_amount_per_round_last_update + SECONDS_PER_MONTH {
            return Ok(());
        }

        // Is there update left to distribute? Should never happen given the % based decay rate
        if self.current_month_emission_amount_per_round == 0 {
            return Ok(());
        }

        let decay_rate = Self::LP_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE;

        let decay_amount = math::checked_as_u64(
            (self.current_month_emission_amount_per_round as u128 * decay_rate as u128)
                / Cortex::RATE_POWER,
        )?;

        self.current_month_emission_amount_per_round = self
            .current_month_emission_amount_per_round
            .saturating_sub(decay_amount);

        self.emission_amount_per_round_last_update = current_time;

        self.months_elapsed_since_inception += 1;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    #[test]
    fn test_apply_emission_filter_fixed_cases() {
        let mut staking = Staking {
            lm_emission_potentiometer_bps: Staking::LM_EMISSION_POTENTIOMETER_BASELINE_BPS, // 100%
            ..Staking::default()
        };

        let initial_amount: u64 = 1_000_000;
        // Test with baseline potentiometer value
        let new_amount = staking.apply_emission_filter(initial_amount).unwrap();
        assert_eq!(new_amount, initial_amount);

        let initial_amount = 100;
        // Test with baseline potentiometer value
        let new_amount = staking.apply_emission_filter(initial_amount).unwrap();
        assert_eq!(new_amount, initial_amount);

        // Test with 50% potentiometer value
        staking.lm_emission_potentiometer_bps = 5_000;
        let new_amount = staking.apply_emission_filter(initial_amount).unwrap();
        assert_eq!(new_amount, 50);

        // Test with 0% potentiometer value (should emit 0)
        staking.lm_emission_potentiometer_bps = 0;
        let new_amount = staking.apply_emission_filter(initial_amount).unwrap();
        assert_eq!(new_amount, 0);

        // Test with initial amount 0 (should always emit 0)
        let initial_amount = 0;
        staking.lm_emission_potentiometer_bps = Staking::LM_EMISSION_POTENTIOMETER_BASELINE_BPS; // Reset to 100%

        let new_amount = staking.apply_emission_filter(initial_amount).unwrap();
        assert_eq!(new_amount, 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]

        #[test]
        fn test_apply_emission_filter_proptest(initial_amount in 0u64..=u64::MAX / Staking::LM_EMISSION_POTENTIOMETER_MAX_BPS as u64, lm_emission_potentiometer_bps in Staking::LM_EMISSION_POTENTIOMETER_MIN_BPS..=Staking::LM_EMISSION_POTENTIOMETER_MAX_BPS) { // Prevents overflow with the / 20_000
            let staking = Staking {
                lm_emission_potentiometer_bps,
                ..Staking::default()
            };

            // Apply the emission filter with generated values
            let new_amount = staking.apply_emission_filter(initial_amount).unwrap();

            match lm_emission_potentiometer_bps.cmp(&Staking::LM_EMISSION_POTENTIOMETER_BASELINE_BPS) {
                std::cmp::Ordering::Less => {
                    // Ensure new_amount is never greater than initial_amount when lm_staking_lm_emission_potentiometer_bps is less than 10_000
                    prop_assert!(new_amount <= initial_amount);
                }
                std::cmp::Ordering::Greater => {
                    // Ensure new_amount is never lower than initial_amount when lm_staking_lm_emission_potentiometer_bps is greater than 10_000
                    prop_assert!(new_amount >= initial_amount);
                }
                std::cmp::Ordering::Equal => {
                    // Ensure new_amount is equal to initial_amount when lm_staking_lm_emission_potentiometer_bps is 10_000
                    prop_assert_eq!(new_amount, initial_amount);
                }
            }

            // Ensure new_amount is 0 if initial_amount is 0 or lm_staking_lm_emission_potentiometer_bps is 0
            if initial_amount == 0 || lm_emission_potentiometer_bps == 0 {
                prop_assert_eq!(new_amount, 0);
            }

        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_update_lm_emission_amount_per_round_for_lp_staking_if_needed(
            current_unix_timestamp in 1704406573..=1733350573i64, // ~12 months period
            months_elapsed_since_inception in 0u16..48,
            current_month_emission_amount_per_round in 1u64..=u64::MAX / 100,
        ) {
            let mut staking = Staking {
                months_elapsed_since_inception,
                emission_amount_per_round_last_update: 1704406573,
                current_month_emission_amount_per_round,
                staking_type: StakingType::LP.into(),
                ..Staking::default()
            };

            // Calculate expected decay rate based on elapsed months
            let expected_decay_rate = Staking::LP_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE;

                staking.update_lm_emission_amount_per_round_for_lp_staking_if_needed(current_unix_timestamp).unwrap();

            // Calculate expected new amount
            let decay_amount = (current_month_emission_amount_per_round as u128 * expected_decay_rate as u128) / Cortex::RATE_POWER;
            // Based if the update is needed or not, calculate the expected amount
            let expected_amount = if current_unix_timestamp - staking.emission_amount_per_round_last_update > SECONDS_PER_MONTH {
                current_month_emission_amount_per_round.saturating_sub(decay_amount as u64)
            } else {
                staking.current_month_emission_amount_per_round
            };

            // Assert that the update matches expectations
            prop_assert_eq!(staking.current_month_emission_amount_per_round, expected_amount);
        }

        #[test]
        fn test_update_lm_emission_amount_per_round_for_lm_staking_if_needed_year_one(
            current_unix_timestamp in 1704406573..=1733350573i64, // ~12 months period
            months_elapsed_since_inception in 0u16..48,
            current_month_emission_amount_per_round in 1u64..=u64::MAX / 100,
        ) {
            let mut staking = Staking {
                months_elapsed_since_inception,
                emission_amount_per_round_last_update: 1704406573,
                current_month_emission_amount_per_round,
                staking_type: StakingType::LM.into(),
                ..Staking::default()
            };

            // Calculate expected decay rate based on elapsed months
            let expected_decay_rate = Staking::LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y1;

            staking.update_lm_emission_amount_per_round_for_lm_staking_if_needed(current_unix_timestamp).unwrap();


            // Calculate expected new amount
            let decay_amount = (current_month_emission_amount_per_round as u128 * expected_decay_rate as u128) / Cortex::RATE_POWER;
            // Based if the update is needed or not, calculate the expected amount
            let expected_amount = if current_unix_timestamp - staking.emission_amount_per_round_last_update > SECONDS_PER_MONTH {
                current_month_emission_amount_per_round.saturating_sub(decay_amount as u64)
            } else {
                staking.current_month_emission_amount_per_round
            };

            // Assert that the update matches expectations
            prop_assert_eq!(staking.current_month_emission_amount_per_round, expected_amount);
        }

        #[test]
        fn test_update_lm_emission_amount_per_round_for_lm_staking_if_needed_year_two(
            current_unix_timestamp in 1704406573..=1733350573i64, // ~12 months period
            months_elapsed_since_inception in 0u16..48,
            current_month_emission_amount_per_round in 1u64..=u64::MAX / 100,
        ) {
            let mut staking = Staking {
                months_elapsed_since_inception,
                emission_amount_per_round_last_update: 1704406573,
                current_month_emission_amount_per_round,
                staking_type: StakingType::LM.into(),
                ..Staking::default()
            };

            // Calculate expected decay rate based on elapsed months
            let expected_decay_rate = Staking::LM_STAKING_REWARDS_EMISSION_MONTHLY_DECAY_RATE_Y2;

            staking.update_lm_emission_amount_per_round_for_lm_staking_if_needed(current_unix_timestamp).unwrap();


            // Calculate expected new amount
            let decay_amount = (current_month_emission_amount_per_round as u128 * expected_decay_rate as u128) / Cortex::RATE_POWER;
            // Based if the update is needed or not, calculate the expected amount
            let expected_amount = if current_unix_timestamp - staking.emission_amount_per_round_last_update > SECONDS_PER_MONTH {
                current_month_emission_amount_per_round.saturating_sub(decay_amount as u64)
            } else {
                staking.current_month_emission_amount_per_round
            };

            // Assert that the update matches expectations
            prop_assert_eq!(staking.current_month_emission_amount_per_round, expected_amount);
        }
    }
}
