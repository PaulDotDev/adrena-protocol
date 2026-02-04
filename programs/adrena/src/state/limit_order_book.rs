use {
    super::{oracle::OraclePrice, position::Side},
    crate::error::AdrenaError,
    anchor_lang::prelude::*,
    borsh::{BorshDeserialize, BorshSerialize},
    bytemuck::{Pod, Zeroable},
};

pub const MAX_LIMIT_ORDERS: usize = 16;

#[derive(
    Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
)]
#[repr(C)]
pub struct LimitOrder {
    pub id: u64,
    pub trigger_price: u64,
    pub limit_price: u64, // 0 means no slippage
    pub custody: Pubkey,
    pub collateral_custody: Pubkey,
    pub side: u8,
    pub initialized: u8,
    pub is_limit_price_set: u8,
    pub _padding: [u8; 5],
    pub amount: u64,
    pub leverage: u32,
    pub _padding2: [u8; 4],
}

#[account(zero_copy)]
#[derive(Default, Debug, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct LimitOrderBook {
    pub initialized: u8,
    pub bump: u8,
    pub registered_limit_order_count: u8,
    pub _padding: [u8; 5], // Adjusted padding to match the size
    pub owner: Pubkey,
    pub limit_orders: [LimitOrder; MAX_LIMIT_ORDERS],
    pub escrowed_lamports: u64,
}

impl LimitOrder {
    pub fn get_side(&self) -> Side {
        // Consider value in the struct always good
        Side::try_from(self.side).unwrap()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized != 0
    }

    pub fn is_limit_price_set(&self) -> bool {
        self.limit_price != 0
    }

    // Returns yes if the order can be executed as it meets the conditions
    pub fn is_executable(&self, token_trade_price: &OraclePrice, custody: &Pubkey) -> bool {
        if self.custody != *custody {
            return false;
        }

        match self.get_side() {
            Side::Long => {
                if token_trade_price.price > self.trigger_price {
                    return false;
                }

                if self.is_limit_price_set() {
                    return token_trade_price.price >= self.limit_price;
                }

                true
            }
            Side::Short => {
                if token_trade_price.price < self.trigger_price {
                    return false;
                }

                if self.is_limit_price_set() {
                    return token_trade_price.price <= self.limit_price;
                }

                true
            }
            _ => false,
        }
    }
}

impl LimitOrderBook {
    pub const LEN: usize = 8 + std::mem::size_of::<LimitOrderBook>();

    /// Adds a limit order to the book, returns an error if no spots are available.
    #[allow(clippy::too_many_arguments)]
    pub fn add_limit_order(
        &mut self,
        id: u64,
        trigger_price: u64,
        limit_price: u64,
        custody: Pubkey,
        collateral_custody: Pubkey,
        side: u8,
        amount: u64,
        leverage: u32,
    ) -> Result<()> {
        let spot = self.limit_orders.iter().position(|p| p.amount == 0);
        match spot {
            Some(index) => {
                self.limit_orders[index] = LimitOrder {
                    id,
                    trigger_price,
                    limit_price,
                    custody,
                    collateral_custody,
                    side,
                    initialized: true as u8,
                    amount,
                    leverage,
                    ..LimitOrder::default()
                };

                self.registered_limit_order_count += 1;

                Ok(())
            }
            None => Err(AdrenaError::InvalidLimitOrderState.into()),
        }
    }

    pub fn get_limit_order(&self, id: u64) -> Result<&LimitOrder> {
        self.limit_orders
            .iter()
            .find(|p| p.id == id)
            .ok_or(AdrenaError::InvalidLimitOrderState.into())
    }

    /// Removes a limit order from the book, returns an error if the order is not found.
    pub fn remove_limit_order(&mut self, id: u64) -> Result<()> {
        let token_id = self.limit_orders.iter().position(|p| p.id == id);

        match token_id {
            Some(index) => {
                self.limit_orders[index] = LimitOrder::default();
                self.registered_limit_order_count -= 1;
                Ok(())
            }
            None => Err(AdrenaError::InvalidLimitOrderState.into()),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized != 0
    }

    // tell if at least 1 limit order escrow a specific collateral
    pub fn is_collateral_escrowed(&self, collateral_custody: &Pubkey) -> bool {
        self.limit_orders
            .iter()
            .any(|x| x.collateral_custody == *collateral_custody)
    }
}
