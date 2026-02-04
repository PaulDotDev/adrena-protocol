use anchor_lang::prelude::*;

#[account]
#[derive(Default, Debug)]
pub struct VestRegistry {
    pub bump: u8,
    pub vests: Vec<Pubkey>,
    // Currently locked up in vests
    pub vesting_token_amount: u64,
    // Claimed (stat)
    pub vested_token_amount: u64,
}

impl VestRegistry {
    // Calculate size manually, else std::mem::size_of::<VestRegistry>() will count 24 bytes for the empty vector
    pub const LEN: usize = 8 // Anchor Discriminator 
                            + std::mem::size_of::<u8>()
                            + std::mem::size_of::<u64>() * 2
                            + 4 /* empty vector */;

    pub fn size(&self) -> usize {
        Self::LEN + self.vests.len() * std::mem::size_of::<Pubkey>()
    }
}
