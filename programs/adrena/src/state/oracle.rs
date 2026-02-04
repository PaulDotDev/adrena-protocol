use {
    super::chaos_labs_oracle::ChaosLabsBatchPrices,
    crate::{
        error::AdrenaError, math, state::cortex::Cortex, utils::limited_string::LimitedString,
    },
    anchor_lang::prelude::*,
    bytemuck::{Pod, Zeroable},
    std::collections::HashSet,
};

pub const ORACLE_EXPONENT_SCALE: i32 = -9;
pub const ORACLE_PRICE_SCALE: u128 = 1_000_000_000;
const ORACLE_MAX_PRICE: u64 = (1 << 28) - 1;
pub const STALENESS: i64 = 15; // in seconds

pub const MAX_ORACLE_PRICES_COUNT: usize = 20;

#[account(zero_copy)]
#[derive(Default, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct Oracle {
    pub bump: u8,
    pub _padding: [u8; 7],
    pub updated_at: i64,
    pub prices: [OraclePrice; MAX_ORACLE_PRICES_COUNT],
}

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Zeroable, Pod,
)]
#[repr(C)]
pub struct OraclePrice {
    pub price: u64,
    pub confidence: u64,
    pub timestamp: i64,
    pub exponent: i32,
    pub chaos_labs_feed_id: u8,
    pub _padding: [u8; 3],
    pub name: LimitedString,
}

impl Oracle {
    pub const LEN: usize = 8 + std::mem::size_of::<Oracle>();

    // Includes a check for stale prices
    pub fn get_oracle_price(&self, name: LimitedString, current_time: i64) -> Result<OraclePrice> {
        match self.prices.iter().find(|p| p.name == name) {
            Some(price) => {
                price.check_price_staleness(current_time)?;
                Ok(*price)
            }
            None => {
                msg!("Missing price for {}", name);
                Err(AdrenaError::MissingOraclePrice.into())
            }
        }
    }

    // READONLY - Does not mutate the oracle_prices account - used by views
    // Verify the prices and loads specific ones in memory
    pub fn get_up_to_date_prices_readonly(
        &self,
        prices: &ChaosLabsBatchPrices,
        names: Vec<LimitedString>,
        current_time: i64,
    ) -> Result<Vec<OraclePrice>> {
        let mut oracle_prices = Vec::<OraclePrice>::new();

        for name in names.iter() {
            let stored_price = self.prices.iter().find(|p| p.name == *name).unwrap();
            let new_price = prices
                .prices
                .iter()
                .find(|p| p.feed_id == stored_price.chaos_labs_feed_id);

            match new_price {
                Some(new_price) => {
                    // If the new price is newer, use it
                    if new_price.timestamp > stored_price.timestamp {
                        let mut c = *stored_price;

                        c.timestamp = new_price.timestamp;
                        c.price = new_price.price;

                        oracle_prices.push(c);
                    } else {
                        // If the new price is older, use the stored price
                        oracle_prices.push(*stored_price);
                    }
                }
                None => {
                    // If there are no new prices, use the stored price
                    oracle_prices.push(*stored_price);
                }
            }
        }

        // Now make sure no price is stale
        for price in oracle_prices.iter() {
            price.check_price_staleness(current_time)?;
        }

        Ok(oracle_prices)
    }

    // Return true if the store prices were updated
    pub fn verify_and_update_prices(
        &mut self,
        prices: &ChaosLabsBatchPrices,
        current_time: i64,
    ) -> Result<bool> {
        prices.verify_signature()?;

        // Keep track of provided feeds to make sure all the prices are provided as parameter
        let mut received_feed_ids = HashSet::with_capacity(prices.prices.len());

        let mut did_update = false;

        for new_price in &prices.prices {
            received_feed_ids.insert(new_price.feed_id);

            let mut found = false;

            for stored_price in &mut self.prices {
                if stored_price.chaos_labs_feed_id == new_price.feed_id {
                    if stored_price.timestamp < new_price.timestamp {
                        stored_price.timestamp = new_price.timestamp;
                        stored_price.price = new_price.price;

                        // Replace Pyth confidence to protect LP from MEV (except for USDC)
                        if stored_price.chaos_labs_feed_id != 5 {
                            stored_price.confidence = math::checked_as_u64(
                                new_price.price as u128 * 25 / Cortex::BPS_POWER,
                            )?;
                        } else {
                            #[cfg(not(feature = "test"))]
                            {
                                stored_price.confidence = 0;
                            }
                        }

                        did_update = true;
                    }

                    // Make sure the given price exists in the stored prices
                    found = true;
                    break;
                }
            }

            require!(found, AdrenaError::InvalidOraclePrice);
        }

        if did_update {
            self.updated_at = current_time;
        }

        // Make sure all required feeds were present
        for expected in &self.prices {
            // Skip if the price is not set
            if expected.price == 0 {
                continue;
            }

            if !received_feed_ids.contains(&expected.chaos_labs_feed_id) {
                msg!("Missing price for {}", expected.name);
            }

            require!(
                received_feed_ids.contains(&expected.chaos_labs_feed_id),
                AdrenaError::MissingOraclePrice
            );
        }

        Ok(did_update)
    }
}

impl OraclePrice {
    pub fn low(&self) -> Self {
        Self {
            price: self.price - self.confidence,
            exponent: self.exponent,
            confidence: 0,
            timestamp: self.timestamp,
            chaos_labs_feed_id: self.chaos_labs_feed_id,
            ..Default::default()
        }
    }

    pub fn high(&self) -> Self {
        Self {
            price: self.price + self.confidence,
            exponent: self.exponent,
            confidence: 0,
            timestamp: self.timestamp,
            chaos_labs_feed_id: self.chaos_labs_feed_id,
            ..Default::default()
        }
    }

    // Converts token amount to USD with implied USD_DECIMALS decimals
    pub fn get_asset_amount_usd(&self, token_amount: u64, token_decimals: u8) -> Result<u64> {
        if token_amount == 0 || self.price == 0 {
            return Ok(0);
        }

        math::checked_decimal_mul(
            token_amount,
            -(token_decimals as i32),
            self.price,
            self.exponent,
            -(Cortex::USD_DECIMALS as i32),
        )
    }

    // Converts USD amount with implied USD_DECIMALS decimals to token amount
    pub fn get_token_amount(&self, asset_amount_usd: u64, token_decimals: u8) -> Result<u64> {
        if asset_amount_usd == 0 || self.price == 0 {
            return Ok(0);
        }

        math::checked_decimal_div(
            asset_amount_usd,
            -(Cortex::USD_DECIMALS as i32),
            self.price,
            self.exponent,
            -(token_decimals as i32),
        )
    }

    /// Returns price with mantissa normalized to be less than ORACLE_MAX_PRICE
    pub fn normalize(&self) -> Result<OraclePrice> {
        let mut p = self.price;
        let mut e = self.exponent;

        while p > ORACLE_MAX_PRICE {
            p /= 10;
            e += 1;
        }

        Ok(OraclePrice {
            price: p,
            exponent: e,
            confidence: self.confidence,
            timestamp: self.timestamp,
            chaos_labs_feed_id: self.chaos_labs_feed_id,
            ..Default::default()
        })
    }

    pub fn scale_to_exponent(&self, target_exponent: i32) -> Result<OraclePrice> {
        if target_exponent == self.exponent {
            return Ok(*self);
        }

        Ok(OraclePrice {
            price: math::scale_to_exponent(self.price, self.exponent, target_exponent)?,
            exponent: target_exponent,
            confidence: self.confidence,
            timestamp: self.timestamp,
            chaos_labs_feed_id: self.chaos_labs_feed_id,
            ..Default::default()
        })
    }

    pub fn check_price_staleness(&self, current_time: i64) -> Result<()> {
        require!(
            self.timestamp + STALENESS >= current_time,
            AdrenaError::StaleOraclePrice
        );

        Ok(())
    }
}
