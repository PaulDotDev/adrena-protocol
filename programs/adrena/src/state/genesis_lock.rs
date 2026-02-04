use {
    super::cortex::Cortex,
    crate::{error::AdrenaError, math},
    anchor_lang::prelude::*,
    std::str::FromStr,
};

// temporary hack waiting to update Anchor. Fixed in 0.29.0
const RESERVED_GRANTS_COUNT: usize = 43;

#[account(zero_copy)]
#[derive(Debug)]
#[repr(C)]
pub struct GenesisLock {
    pub bump: u8,
    pub has_transitioned_to_fully_public: u8,
    pub has_completed_otc_in: u8,
    pub has_completed_otc_out: u8,
    pub _padding: [u8; 4],
    //
    // Duration in second of the genesis lock campaign (during which users can deposit funds)
    pub campaign_duration: i64,
    // Duration in seconds determining how long before the reserved grants become public
    pub reserved_grant_duration: i64,
    // Timestamp of the starting date of the genesis campaign
    pub campaign_start_date: i64,
    // the amount for the public
    pub public_amount: u64,
    // the amount for insiders
    pub reserved_amount: u64,
    pub public_amount_claimed: u64,
    pub reserved_amount_claimed: u64,

    // Array containing the owners and amount for the insider allocation
    pub reserved_grant_owners: [Pubkey; RESERVED_GRANTS_COUNT],
    pub reserved_grant_amounts: [u64; RESERVED_GRANTS_COUNT],
    //
    pub _padding_unsafe: [u8; 8],
}

impl GenesisLock {
    pub const RESERVED_GRANTS_COUNT: usize = RESERVED_GRANTS_COUNT;
    pub const USDC_DECIMALS: u8 = 6;
    pub const LEN: usize = 8 + std::mem::size_of::<GenesisLock>();
    pub const CAMPAIGN_PUBLIC_USDC_AMOUNT: u64 = 5_000_000 * 10u64.pow(Self::USDC_DECIMALS as u32);
    pub const CAMPAIGN_RESERVED_USDC_AMOUNT: u64 =
        5_000_000 * 10u64.pow(Self::USDC_DECIMALS as u32);
    pub const CAMPAIGN_REWARDS_ADX_AMOUNT: u64 = 50_000_000 * 10u64.pow(Cortex::LM_DECIMALS as u32); // 5% of supply distributed for the campaign
    pub const CAMPAIGN_CLAIM_DURATION_DAYS: u32 = 180; // 6 months
    pub const CAMPAIGN_CLAIM_DURATION_SECONDS: i64 =
        Self::CAMPAIGN_CLAIM_DURATION_DAYS as i64 * 24 * 60 * 60;

    pub fn get_time() -> Result<i64> {
        let time = solana_program::sysvar::clock::Clock::get()?.unix_timestamp;

        if time > 0 {
            Ok(time)
        } else {
            Err(ProgramError::InvalidAccountData.into())
        }
    }

    pub fn init_with_hardcoded_value(&mut self) -> Result<()> {
        self.public_amount = Self::CAMPAIGN_PUBLIC_USDC_AMOUNT;
        self.reserved_amount = Self::CAMPAIGN_RESERVED_USDC_AMOUNT;
        self.public_amount_claimed = 0;
        self.reserved_amount_claimed = 0;

        // One time flag once the reserved_grants_end_date is reached
        self.has_transitioned_to_fully_public = 0;

        self.reserved_grant_owners = [
            Pubkey::from_str("DgyBddqzSxZW9tYwiqJyV4eKpBhez2ZScjyZqvvKvgmw").unwrap(),
            Pubkey::from_str("8ge99cGUXP9Lqu7s2DEqg4X7Vhmt3T9EQnMLVhT5Qz85").unwrap(),
            Pubkey::from_str("EzbYcq7t65sUMxcENshiUBXgh7o7PV8SHmj9o5D4ufqN").unwrap(),
            Pubkey::from_str("AC1ffKPNBvebKHty6eczGST6zHjmA397Nzgpn7gCj1hf").unwrap(),
            Pubkey::from_str("Bibox49AvjhLEWCypKKcRh2yjb6djTsV4vDF4bBHc95P").unwrap(),
            Pubkey::from_str("E5ETMpSXjxXVm8TXPrEdTuP7hrXQSdXN9e3HHkNJ9UaE").unwrap(),
            Pubkey::from_str("39YD5rZrsz8mSdtyVCfwaG2cAh5Amq4wDAgqZEtgXLRj").unwrap(),
            Pubkey::from_str("49nDuD2RdagC1Mh89HNS8sQNMWUPEHmeihNeCTWkAKZx").unwrap(),
            Pubkey::from_str("A1RmrB86Fq86xDC6N3JkatgGA2Qa7XqiFQzWWnh4vZXV").unwrap(),
            Pubkey::from_str("C3DGZ4yMo6PU1vJ47hLJAxDoUjJpms6TFPTnKFGwMWuL").unwrap(),
            Pubkey::from_str("GSaScBcrZxcts8DN6xTwB3yz36FEyei6gDv3vFXgeivm").unwrap(),
            Pubkey::from_str("BYn4KW5naqQa6s6Gns9W9apN6SQyFxnpei3EAXv5VdGF").unwrap(),
            Pubkey::from_str("2uW32nbjGAXf49G4KTnESGqvrMfH8Sj2DmtZYEFcjGMu").unwrap(),
            Pubkey::from_str("6ALGMay8AmcywGAX72ho7JbSucD7zeh4hwMVyXDb9zgy").unwrap(),
            Pubkey::from_str("95f5JT9hfCV9a5sivpgQeXSAiVeXjC1aMyxHqGb7oAPf").unwrap(),
            Pubkey::from_str("4ErZWpvajdxwwGsXSdseiupCD6Hc21gKSo8AML6j5ctB").unwrap(),
            Pubkey::from_str("5VnnDb6r4i8aTw2iNTCiHt4DmbpKnPBZQHNp5QFA3pgs").unwrap(),
            Pubkey::from_str("GNeiiy4PChWKxPt9nQLP6jxv2gjQTFGRjioQUBFeKeAY").unwrap(),
            Pubkey::from_str("6TdrY6cZr2gGQbXVRf5KuYxM2d9TNe8o9dKJwesn2s4N").unwrap(),
            Pubkey::from_str("JCzBRMxPoGfnXE85MLZkEFjkxiQCskiGnyb4rqWCZgDj").unwrap(),
            Pubkey::from_str("E7mseB5gT5JnN9sy9jYPFZahXhAi1YSisZ1UtR2TXPwX").unwrap(),
            Pubkey::from_str("DeEMGyr4iKJ184te7pCw8ZF14x4VfMktYrvHtj5BMzUN").unwrap(),
            Pubkey::from_str("97wgj1vXKZMpCZz12inZpWsBBGCQ1foxce2g7jExKAUf").unwrap(),
            Pubkey::from_str("6DRFmQ8bZDSq8n9uoKnto6NVEqMNHKpKAoHqxXX13JW1").unwrap(),
            Pubkey::from_str("5Sp6WK4tuasZu1ZizUHN1uPyJJeAbkJxMxcFaVdWugGH").unwrap(),
            Pubkey::from_str("5Am5nsuKnDmNiAU2owXCnFaz7SUKjYxmBSDrJNGoHazZ").unwrap(),
            Pubkey::from_str("GNUEffMdZRC3coWixBjMRxxgJfTbTEPjuuzKxNvY8jA9").unwrap(),
            Pubkey::from_str("BhbAiR5RDPmM9xN7fZ35dSqKq8g9MPw5JUaZbudFrsfC").unwrap(),
            Pubkey::from_str("DkjHjwPSNWDvmSHY5hB2nWyJthk1RGGecz51C2KqBB7e").unwrap(),
            Pubkey::from_str("EwCrpqGq45ogMWp3U1DhNyeDjFB9TXMTbTRTzo3KMaX7").unwrap(),
            Pubkey::from_str("8LToRkuSHpQoFodWbjY3djhTjAypWpPLjSLU8u6CsprA").unwrap(),
            Pubkey::from_str("Bf6eB4g4akvc4KUPHyBSHfgXumqqhUahGmd33XoFyJ9X").unwrap(),
            Pubkey::from_str("3ZeZZyWKnUcgDZVFsVqhLHau4yR7o9gBQF6gAaMCEVbM").unwrap(),
            Pubkey::from_str("FyWThak5XAkjvD7wJgTPyS8EU4hBui4a4edsxfwk5tT3").unwrap(),
            Pubkey::from_str("BZn3dtZKWi78VbSJ7QCc3rdv7p6WxnUKUxkZskQrHEYN").unwrap(),
            Pubkey::from_str("E6fUx7KC2uu1ShFiHQqV8nLP9Kx5nhiTjqMvseJFYF6S").unwrap(),
            Pubkey::from_str("21GtFuxdtM2VujUQ2Goviz2EdZ6AX28w2faBRmWYG1bb").unwrap(),
            Pubkey::from_str("32fzvh3srTFJdySYVnZ6N3yXDHYPJ2iQkG6VCW1ATVfW").unwrap(),
            Pubkey::from_str("7G9Vsdq2YSFthRTiPmNuFRFRCoX3hpABmaWGHFrM2kPR").unwrap(),
            Pubkey::from_str("GuxAPwbZ5jqt5tKjGUWDUuUM9GhMXopdSmkJPn1eQWBU").unwrap(),
            Pubkey::from_str("DG1SoPvhsP5hNv2Xnw65Qp5uddVdn7rc2UecCNeNLbsR").unwrap(),
            Pubkey::from_str("Ehw3RgdbzyAHHddgSv7F6f3WtYhyBtRg2ropQaWGgwcL").unwrap(),
            Pubkey::from_str("3HSdxAiJbtstE9GW1veVn859iKut5kGsxXNaZQ4AQDsc").unwrap(),
        ];

        self.reserved_grant_amounts = [
            math::to_token_amount(635_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(500_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(500_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(500_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(350_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(300_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(250_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(100_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(200_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(200_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(100_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(200_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(100_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(100_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(97_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(79_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(50_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(40_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(40_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(40_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(30_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(25_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(20_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(20_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(20_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(15_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(39_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(10_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(10_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(5_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(5_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(5_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(4_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(1_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
            math::to_token_amount(10_000.0, GenesisLock::USDC_DECIMALS).unwrap(),
        ];

        Ok(())
    }

    // Check if current time is within the genesis lock campaign
    pub fn is_campaign_open(&self) -> Result<bool> {
        let time = Self::get_time()?;

        Ok(time >= self.campaign_start_date && time <= self.get_campaign_end_date())
    }

    pub fn is_otc_in_completed(&self) -> bool {
        self.has_completed_otc_in == 1
    }

    pub fn is_otc_out_completed(&self) -> bool {
        self.has_completed_otc_out == 1
    }

    // Timestamp at which the reward distribution period ends
    pub fn reward_distribution_period_end_date(&self) -> i64 {
        self.get_campaign_end_date() + Self::CAMPAIGN_CLAIM_DURATION_SECONDS
    }

    pub fn has_campaign_ended(&self) -> Result<bool> {
        let time = Self::get_time()?;
        Ok(time >= self.get_campaign_end_date())
    }

    pub fn get_campaign_end_date(&self) -> i64 {
        self.campaign_start_date + self.campaign_duration
    }

    pub fn grant_reservation_period_ended(&self) -> Result<bool> {
        let time = Self::get_time()?;

        Ok(time > self.campaign_start_date + self.reserved_grant_duration)
    }

    // Updates the reserved grant amount for the provided pubkey, if found and sufficient.
    pub fn claim_reserved_grant_amount(&mut self, pubkey: &Pubkey, amount: u64) -> Result<u64> {
        // If the private grant period ended, return 0 (and attempt to do the transition of the remains to public grants)
        if self.grant_reservation_period_ended()? {
            if !self.has_transitioned_to_fully_public() {
                self.transition_to_fully_public()?;
            }

            return Ok(0);
        }

        for i in 0..Self::RESERVED_GRANTS_COUNT {
            let amount_available = self.reserved_grant_amounts[i];

            if amount <= amount_available && self.reserved_grant_owners[i] == *pubkey {
                self.reserved_grant_amounts[i] -= amount;
                self.reserved_amount_claimed += amount;

                return Ok(amount);
            }
        }

        Ok(0)
    }

    // All the unclaimed reserved grants are now transferred to the public to be claimable by anyone
    fn transition_to_fully_public(&mut self) -> Result<()> {
        msg!("Transitioning to fully public");

        self.public_amount += self.reserved_amount - self.reserved_amount_claimed;
        self.reserved_amount = self.reserved_amount_claimed;
        self.has_transitioned_to_fully_public = 1;

        Ok(())
    }

    pub fn has_transitioned_to_fully_public(&self) -> bool {
        self.has_transitioned_to_fully_public == 1
    }

    // Update the public grant amount for the provided amount, if sufficient.
    // Returns true if the update was successful, false otherwise.
    pub fn claim_public_grant_amount(&mut self, amount: u64) -> Result<u64> {
        let public_amount_available = self.public_amount - self.public_amount_claimed;

        require!(
            public_amount_available > 0,
            AdrenaError::GenesisLockCampaignFullySubscribed
        );

        if public_amount_available >= amount {
            self.public_amount_claimed += amount;

            Ok(amount)
        } else {
            self.public_amount_claimed += public_amount_available;

            Ok(public_amount_available)
        }
    }

    pub fn sanity_check(&self) -> Result<()> {
        let total_campaign_amount: u64 =
            Self::CAMPAIGN_PUBLIC_USDC_AMOUNT + Self::CAMPAIGN_RESERVED_USDC_AMOUNT;

        if self.public_amount + self.reserved_amount > total_campaign_amount
            || self.public_amount_claimed > self.public_amount
            || self.reserved_amount_claimed > self.reserved_amount
            || self.reserved_amount_claimed + self.public_amount_claimed > total_campaign_amount
        {
            return Err(AdrenaError::InvalidGenesisLockState.into());
        }

        if !self.has_transitioned_to_fully_public() {
            // make sure total in reserved_grant_amounts is equal to reserved_amount
            let mut total_reserved_grant_amounts = 0;

            for i in 0..Self::RESERVED_GRANTS_COUNT {
                total_reserved_grant_amounts += self.reserved_grant_amounts[i];
            }

            msg!(
                "Total reserved grant amounts: {}",
                total_reserved_grant_amounts
            );
            msg!("Reserved amount: {}", self.reserved_amount);

            require!(
                total_reserved_grant_amounts == self.reserved_amount - self.reserved_amount_claimed,
                AdrenaError::InvalidGenesisLockState
            );
        }

        Ok(())
    }

    // Amount of LM token extra reward per ALP (staked), per seconds, to be distributed as rewards
    // NOTE: reward_rate_per_alp_per_second suffers precision loss, that is deemed acceptable.
    pub fn reward_rate_per_alp_per_second(genesis_liquidity_alp_amount: u64) -> u128 {
        if genesis_liquidity_alp_amount == 0 {
            return 0;
        }

        let reward_rate_per_second = (Self::CAMPAIGN_REWARDS_ADX_AMOUNT as u128
            * Cortex::RATE_POWER)
            / Self::CAMPAIGN_CLAIM_DURATION_SECONDS as u128;

        reward_rate_per_second / (genesis_liquidity_alp_amount as u128)
    }
}
