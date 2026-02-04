use {
    crate::{
        adapters,
        error::AdrenaError,
        instructions::{
            get_swap_amount_and_fees::GetSwapAmountAndFeesParams, BucketName,
            GetEntryPriceAndFeeParams, SwapParams,
        },
        math,
        program::Adrena,
        ClosePositionLongParams, ClosePositionShortParams, IncreasePositionLongParams,
        IncreasePositionShortParams, OpenPositionLongParams, OpenPositionShortParams,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Burn, MintTo, Transfer},
    solana_program::{account_info::AccountInfo, program::invoke_signed, system_instruction},
};

pub const HOURS_PER_DAY: i64 = 24;
pub const SECONDS_PER_HOURS: i64 = 3600;

#[derive(PartialEq, Copy, Clone, Default, Debug)]
pub enum CortexInitializationStep {
    #[default]
    NotCreated = 0,
    Step1 = 1,
    Step2 = 2,
    Step3 = 3,
    Initialized = 4,
}

impl From<CortexInitializationStep> for u8 {
    fn from(val: CortexInitializationStep) -> Self {
        match val {
            CortexInitializationStep::NotCreated => 0,
            CortexInitializationStep::Step1 => 1,
            CortexInitializationStep::Step2 => 2,
            CortexInitializationStep::Step3 => 3,
            CortexInitializationStep::Initialized => 4,
        }
    }
}

impl TryFrom<u8> for CortexInitializationStep {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => CortexInitializationStep::NotCreated,
            1 => CortexInitializationStep::Step1,
            2 => CortexInitializationStep::Step2,
            3 => CortexInitializationStep::Step3,
            4 => CortexInitializationStep::Initialized,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidCortexState)?,
        })
    }
}

#[account(zero_copy)]
#[derive(Default, Debug)]
#[repr(C)]
pub struct Cortex {
    pub bump: u8,
    pub transfer_authority_bump: u8,
    pub lm_token_bump: u8,
    pub governance_token_bump: u8,
    pub initialized: u8, // CortexInitializationStep
    pub fee_conversion_decimals: u8,
    pub _padding: [u8; 2],
    pub lm_token_mint: Pubkey,
    //
    pub inception_time: i64,
    pub admin: Pubkey,
    //
    // Depending of the context:
    // - convert collected fees into this mint
    // - distribute rewards in this mint
    //
    pub fee_redistribution_mint: Pubkey,
    pub protocol_fee_recipient: Pubkey, // a wallet from the DAO that will handle buybacks
    //
    pub pools: [Pubkey; 4],
    pub user_profiles_count: u64,
    //
    // Governance
    //
    pub governance_program: Pubkey,
    pub governance_realm: Pubkey,
    //
    // LM Token buckets' allocations
    //
    pub core_contributor_bucket_allocation: u64,
    pub foundation_bucket_allocation: u64,
    pub ecosystem_bucket_allocation: u64,
    //
    // Buckets stats
    //
    pub core_contributor_bucket_vested_amount: u64,
    pub core_contributor_bucket_minted_amount: u64,
    pub foundation_bucket_vested_amount: u64,
    pub foundation_bucket_minted_amount: u64,
    pub ecosystem_bucket_vested_amount: u64,
    pub ecosystem_bucket_minted_amount: u64,
    //
    // genesis Lock campaign contributions
    pub genesis_liquidity_alp_amount: u64,
    // Unique ID counter for positions (incremented with wrapping add, looping)
    pub unique_position_id_counter: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct ExitPriceAndFee {
    pub price: u64,
    pub fee: u64,
    pub amount_out: u64,
    pub profit_usd: u64,
    pub loss_usd: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct AmountAndFee {
    pub amount: u64,
    pub fee: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct NewPositionPricesAndFee {
    pub entry_price: u64,
    pub liquidation_price: u64,
    pub exit_fee: u64,
    pub liquidation_fee: u64,
    pub size: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct OpenPositionWithSwapAmountAndFees {
    pub entry_price: u64,
    pub liquidation_price: u64,
    pub swap_fee_in: u64,
    pub swap_fee_out: u64,
    pub exit_fee: u64,
    pub liquidation_fee: u64,
    pub size: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct SwapAmountAndFees {
    pub amount_out: u64,
    pub fee_in: u64,
    pub fee_out: u64,
}

/// Specific to the codebase, this struct is used to store the profit and loss of a position.
#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct ProfitAndLoss {
    pub profit_usd: u64,
    pub loss_usd: u64,
    // Unrealized
    pub exit_fee: u64,
    pub exit_fee_usd: u64,
    pub borrow_fee_usd: u64,
}

// Represent the fee distribution between:
//
// - ADX stakers
// - ALP holders
// - ALP locked stakers
// - Protocol Fee (external wallet, DAO. For buybacks)
#[derive(Default, Debug, Clone, Copy)]
pub struct FeeDistribution {
    pub lm_stakers_fee: u64,
    pub locked_lp_stakers_fee: u64,
    pub lp_organic_fee: u64,
    pub protocol_fee: u64,
}

/// ------------------------------------------------------
/// General
/// ------------------------------------------------------
impl Cortex {
    pub const LEN: usize = 8 + std::mem::size_of::<Cortex>();
    // BPS
    pub const BPS_DECIMALS: u8 = 4;
    pub const BPS_POWER: u128 = 10u64.pow(Self::BPS_DECIMALS as u32) as u128;
    // RATE
    pub const RATE_POWER: u128 = 10u64.pow(Self::RATE_DECIMALS as u32) as u128;
    pub const RATE_DECIMALS: u8 = 9;
    // Lamports
    pub const AUTOMATION_EXECUTION_FEE: u64 = 300_000;
    //
    // Warning: A low price decimals means a possible loss of precision when interpreting pyth prices
    // Look at the function scale_to_exponent
    pub const PRICE_DECIMALS: u8 = 10;
    //
    pub const USD_DECIMALS: u8 = 6;
    pub const LP_DECIMALS: u8 = Self::USD_DECIMALS;
    pub const LM_DECIMALS: u8 = Cortex::USD_DECIMALS;

    pub const GOVERNANCE_SHADOW_TOKEN_DECIMALS: u8 = Cortex::USD_DECIMALS;

    // Fee distributions, in BPS
    // If Changing theses values, need  to update the get_* function that calculate the fees in "impl Cortex" below
    pub const LP_HOLDERS_FEE_SHARE_AMOUNT: u128 = 7_000; // 70%
    pub const PROTOCOL_FEE_SHARE_AMOUNT: u128 = 1_000; // 10%

    // The minimum amount of collateral that must be posted to open a position
    pub const POSITION_MIN_COLLATERAL_VALUE: u64 = 9 * 10u64.pow(Cortex::USD_DECIMALS as u32); // 9 USD - Using 9 as we use low price, to keep the mental "10$" working for people

    pub fn get_initialized(&self) -> CortexInitializationStep {
        // We consider what's inside the structure safe
        CortexInitializationStep::try_from(self.initialized).unwrap()
    }

    pub fn is_initialized(&self) -> bool {
        self.get_initialized() == CortexInitializationStep::Initialized
    }

    pub fn get_time(&self) -> Result<i64> {
        let time = solana_program::sysvar::clock::Clock::get()?.unix_timestamp;

        if time > 0 {
            Ok(time)
        } else {
            Err(ProgramError::InvalidAccountData.into())
        }
    }

    pub fn is_empty_account(account_info: &AccountInfo) -> Result<bool> {
        Ok(account_info.try_data_is_empty()? || account_info.try_lamports()? == 0)
    }

    pub fn add_pool(&mut self, pool_key: &Pubkey) -> Result<()> {
        let spot = self.pools.iter().position(|p| *p == Pubkey::default());
        match spot {
            Some(index) => {
                self.pools[index] = *pool_key;
                Ok(())
            }
            None => Err(AdrenaError::MaxRegisteredPool.into()),
        }
    }

    pub fn remove_pool(&mut self, pool_key: &Pubkey) -> Result<()> {
        let spot = self.pools.iter().position(|p| *p == *pool_key);

        match spot {
            Some(index) => {
                self.pools[index] = Pubkey::default();
                Ok(())
            }
            None => Err(AdrenaError::InvalidCortexState.into()),
        }
    }

    pub fn get_unique_position_id(&mut self) -> u64 {
        let id = self.unique_position_id_counter;

        self.unique_position_id_counter = self.unique_position_id_counter.wrapping_add(1);

        id
    }
}

/// ------------------------------------------------------
/// Fees
/// ------------------------------------------------------
impl Cortex {
    // While 70% of fees goes to LP, the remaining 30% are split between ADX stakers and the protocol
    // 2/3 for ADX stakers and 1/3 for the protocol
    pub fn get_lm_stakers_fee_from_fee_amount_without_lp_fee(
        &self,
        fee_amount_without_lp_fee: u64,
    ) -> Result<u64> {
        math::checked_as_u64((fee_amount_without_lp_fee as u128 * 2) / 3)
    }

    // While 70% of fees goes to LP, the remaining 30% are split between ADX stakers and the protocol
    // 2/3 for ADX stakers and 1/3 for the protocol
    pub fn get_protocol_fee_from_fee_amount_without_lp_fee(
        &self,
        fee_amount_without_lp_fee: u64,
    ) -> Result<u64> {
        math::checked_as_u64(fee_amount_without_lp_fee as u128 / 3)
    }

    pub fn get_protocol_fee(&self, fee_amount: u64) -> Result<u64> {
        math::checked_as_u64(
            (fee_amount as u128 * Cortex::PROTOCOL_FEE_SHARE_AMOUNT) / Cortex::BPS_POWER,
        )
    }

    // The fee that goes to the LP stakers that are not locked
    // Use get_lp_fee instead, there are no more staked LP starting FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP
    #[deprecated]
    #[allow(deprecated)]
    pub fn get_lp_organic_fee(
        &self,
        fee_amount: u64,
        lp_token_supply: u64,
        lp_staking_current_staking_round_total_stake: u64,
    ) -> Result<u64> {
        let lp_fee = self.get_lp_fee(fee_amount)?;

        let locked_lp_stakers_fee = self.get_locked_lp_stakers_fee(
            lp_fee,
            lp_token_supply,
            lp_staking_current_staking_round_total_stake,
        )?;

        Ok(lp_fee - locked_lp_stakers_fee)
    }

    // The total LP fee (locked + unlocked)
    pub fn get_lp_fee(&self, fee_amount: u64) -> Result<u64> {
        math::checked_as_u64(
            (fee_amount as u128 * Cortex::LP_HOLDERS_FEE_SHARE_AMOUNT) / Cortex::BPS_POWER,
        )
    }

    #[deprecated] // Use get_lp_fee instead, there are no more staked LP starting FULLY_ALP_LIQUID_BREAKPOINT_TIMESTAMP
    pub fn get_locked_lp_stakers_fee(
        &self,
        lp_fee: u64,
        lp_token_supply: u64,
        lp_staking_current_staking_round_total_stake: u64,
    ) -> Result<u64> {
        if lp_fee == 0 {
            return Ok(0);
        }

        let total_lp_holders_shares =
            lp_token_supply + lp_staking_current_staking_round_total_stake;

        if total_lp_holders_shares == 0 {
            return Ok(0);
        }

        let share_per_token =
            (lp_fee as u128 * Cortex::RATE_POWER) / total_lp_holders_shares as u128;

        math::checked_as_u64(
            (share_per_token * lp_staking_current_staking_round_total_stake as u128)
                / Cortex::RATE_POWER,
        )
    }
}

/// ------------------------------------------------------
/// Buckets
/// ------------------------------------------------------
impl Cortex {
    pub fn get_token_amount_left_in_bucket(&self, bucket: BucketName) -> u64 {
        match bucket {
            BucketName::CoreContributor => {
                self.core_contributor_bucket_allocation - self.core_contributor_bucket_minted_amount
            }
            BucketName::Foundation => {
                self.foundation_bucket_allocation - self.foundation_bucket_minted_amount
            }
            BucketName::Ecosystem => {
                self.ecosystem_bucket_allocation - self.ecosystem_bucket_minted_amount
            }
        }
    }

    pub fn get_non_reserved_token_amount_left_in_bucket(&self, bucket: BucketName) -> u64 {
        match bucket {
            BucketName::CoreContributor => {
                self.get_token_amount_left_in_bucket(bucket)
                    - self.core_contributor_bucket_vested_amount
            }
            BucketName::Foundation => {
                self.get_token_amount_left_in_bucket(bucket) - self.foundation_bucket_vested_amount
            }
            BucketName::Ecosystem => {
                self.get_token_amount_left_in_bucket(bucket) - self.ecosystem_bucket_vested_amount
            }
        }
    }

    pub fn update_bucket_vested_amount(&mut self, bucket: BucketName, amount: i64) {
        match bucket {
            BucketName::CoreContributor => {
                self.core_contributor_bucket_vested_amount = if amount >= 0 {
                    self.core_contributor_bucket_vested_amount + amount as u64
                } else {
                    self.core_contributor_bucket_vested_amount - amount.unsigned_abs()
                };
            }
            BucketName::Foundation => {
                self.foundation_bucket_vested_amount = if amount >= 0 {
                    self.foundation_bucket_vested_amount + amount as u64
                } else {
                    self.foundation_bucket_vested_amount - amount.unsigned_abs()
                };
            }
            BucketName::Ecosystem => {
                self.ecosystem_bucket_vested_amount = if amount >= 0 {
                    self.ecosystem_bucket_vested_amount + amount as u64
                } else {
                    self.ecosystem_bucket_vested_amount - amount.unsigned_abs()
                };
            }
        };
    }

    pub fn update_bucket_minted_amount(&mut self, bucket: BucketName, amount: u64) -> Result<()> {
        match bucket {
            BucketName::CoreContributor => {
                self.core_contributor_bucket_minted_amount += amount;

                require!(
                    self.core_contributor_bucket_minted_amount
                        <= self.core_contributor_bucket_allocation,
                    AdrenaError::BucketMintLimit
                );
            }
            BucketName::Foundation => {
                self.foundation_bucket_minted_amount += amount;

                require!(
                    self.foundation_bucket_minted_amount <= self.foundation_bucket_allocation,
                    AdrenaError::BucketMintLimit
                );
            }
            BucketName::Ecosystem => {
                self.ecosystem_bucket_minted_amount += amount;

                require!(
                    self.ecosystem_bucket_minted_amount <= self.ecosystem_bucket_allocation,
                    AdrenaError::BucketMintLimit
                );
            }
        };
        Ok(())
    }
}

/// ------------------------------------------------------
/// Governance
/// ------------------------------------------------------
impl Cortex {
    /// The governance is managed through the program only.
    /// On behalf of users, the program manages their voting power (through Vest and Stake they own).
    /// Depending of the lm_token contained in these accounts and of their voting multiplier, if any, the
    /// program mint new governance token that are own by said Stake/Vest accounts and their voting power are
    /// delegated to the owner (the end user).
    /// This allow flexible voting power with multiplier, de-correlated from the actual lm_token amount held in these
    /// accounts.
    /// Furthermore, this enforces that the governance token is soul-bound to a user, non tradable.
    ///
    /// Updated: Governance is setup with Membership, which allow us to set the owner as the final owner and
    /// avoid delegation of vote (simplify things).
    /// Owner can auto revoke at worse, and to hedge against this we always revoke the min amount between
    /// user voting power and our initial revoke target.
    #[allow(clippy::too_many_arguments)]
    pub fn remove_governing_power<'a>(
        &self,
        transfer_authority: AccountInfo<'a>,
        // The owner of the voting power
        governing_token_owner: AccountInfo<'a>,
        governing_token_owner_record: AccountInfo<'a>,
        // Mint of the shadow governance token (will burn)
        governance_token_mint: AccountInfo<'a>,
        realm: AccountInfo<'a>,
        realm_config: AccountInfo<'a>,
        governing_token_holding: AccountInfo<'a>,
        governance_program: AccountInfo<'a>,
        revoke_amount: u64,
    ) -> Result<()> {
        msg!(
            "Governance - Revoke {} governing power from the owner: {}",
            revoke_amount,
            governing_token_owner.key
        );

        // Revoke tokens (the owner (vest or stake) get burnt the revoked amount of token)
        {
            let authority_seeds: &[&[&[u8]]] =
                &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

            let cpi_accounts = adapters::RevokeGoverningTokens {
                realm: realm.to_account_info(),
                governing_token_holding,
                governing_token_owner_record: governing_token_owner_record.to_account_info(),
                governing_token_mint: governance_token_mint.to_account_info(),
                governing_token_revoke_authority: transfer_authority.to_account_info(),
                realm_config,
                governing_token_owner: governing_token_owner.to_account_info(),
                governing_token_mint_authority: transfer_authority.to_account_info(),
            };

            let cpi_program = governance_program.to_account_info();

            adapters::revoke_governing_token(
                CpiContext::new(cpi_program, cpi_accounts).with_signer(authority_seeds),
                revoke_amount,
            )?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_governing_power<'a>(
        &self,
        transfer_authority: AccountInfo<'a>,
        payer: AccountInfo<'a>,
        governing_token_owner: AccountInfo<'a>,
        governing_token_owner_record: AccountInfo<'a>,
        // Mint of the shadow governance token (will mint)
        governance_token_mint: AccountInfo<'a>,
        realm: AccountInfo<'a>,
        realm_config: AccountInfo<'a>,
        governing_token_holding: AccountInfo<'a>,
        governance_program: AccountInfo<'a>,
        amount: u64,
        additional_signer_seeds: Option<&[&[u8]]>,
        owner_is_signer: bool,
    ) -> Result<()> {
        msg!(
            "Governance - Mint {} governing power to the owner: {}",
            amount,
            governing_token_owner.key
        );

        // Mint tokens in governance for the owner
        {
            let authority_seeds: &[&[u8]] =
                &[b"transfer_authority", &[self.transfer_authority_bump]];

            let cpi_accounts = adapters::DepositGoverningTokens {
                realm: realm.to_account_info(),
                governing_token_mint: governance_token_mint.to_account_info(),
                governing_token_source: governance_token_mint.to_account_info(),
                governing_token_owner: governing_token_owner.to_account_info(),
                governing_token_transfer_authority: transfer_authority,
                payer,
                realm_config,
                governing_token_holding,
                governing_token_owner_record: governing_token_owner_record.to_account_info(),
            };

            // In case the owner is not signer in involved TX (addVest for instance)
            let signers_seeds = match additional_signer_seeds {
                Some(additional_signer_seeds) => vec![authority_seeds, additional_signer_seeds],
                None => vec![authority_seeds],
            };

            let cpi_program = governance_program.to_account_info();

            match owner_is_signer {
                true => adapters::deposit_governing_tokens(
                    CpiContext::new(cpi_program, cpi_accounts).with_signer(&signers_seeds),
                    amount,
                )?,
                false => adapters::deposit_governing_tokens_owner_not_signer(
                    CpiContext::new(cpi_program, cpi_accounts).with_signer(&signers_seeds),
                    amount,
                )?,
            }
        }

        Ok(())
    }
}

/// ------------------------------------------------------
/// Solana CPI Helpers
/// ------------------------------------------------------
impl Cortex {
    pub fn transfer_tokens<'info>(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        msg!(
            "Transfer {} tokens from {} to {}",
            amount,
            from.key(),
            to.key()
        );

        let context = CpiContext::new(
            token_program,
            Transfer {
                from,
                to,
                authority,
            },
        )
        .with_signer(authority_seeds);

        anchor_spl::token::transfer(context, amount)
    }

    pub fn transfer_tokens_from_user<'info>(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let context = CpiContext::new(
            token_program,
            Transfer {
                from,
                to,
                authority,
            },
        );
        anchor_spl::token::transfer(context, amount)
    }

    pub fn mint_tokens<'info>(
        &self,
        mint: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let context = CpiContext::new(
            token_program,
            MintTo {
                mint,
                to,
                authority,
            },
        )
        .with_signer(authority_seeds);

        anchor_spl::token::mint_to(context, amount)
    }

    pub fn burn_tokens<'info>(
        &self,
        mint: AccountInfo<'info>,
        from: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let context = CpiContext::new(
            token_program,
            Burn {
                mint,
                from,
                authority,
            },
        )
        .with_signer(authority_seeds);

        anchor_spl::token::burn(context, amount)
    }

    pub fn burn_tokens_from_user<'info>(
        &self,
        mint: AccountInfo<'info>,
        from: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let context = CpiContext::new(
            token_program,
            Burn {
                mint,
                from,
                authority,
            },
        );

        anchor_spl::token::burn(context, amount)
    }
    pub fn close_token_account<'info>(
        receiver: AccountInfo<'info>,
        token_account: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        seeds: &[&[&[u8]]],
    ) -> Result<()> {
        let cpi_accounts = anchor_spl::token::CloseAccount {
            account: token_account,
            destination: receiver,
            authority,
        };
        let cpi_context = anchor_lang::context::CpiContext::new(token_program, cpi_accounts);

        anchor_spl::token::close_account(cpi_context.with_signer(seeds))
    }

    pub fn transfer_sol_from_owned<'a>(
        program_owned_source_account: AccountInfo<'a>,
        destination_account: AccountInfo<'a>,
        amount: u64,
    ) -> Result<()> {
        **destination_account.try_borrow_mut_lamports()? = destination_account
            .try_lamports()?
            .checked_add(amount)
            .ok_or(ProgramError::InsufficientFunds)?;

        let source_balance = program_owned_source_account.try_lamports()?;
        **program_owned_source_account.try_borrow_mut_lamports()? = source_balance
            .checked_sub(amount)
            .ok_or(ProgramError::InsufficientFunds)?;

        Ok(())
    }

    pub fn transfer_sol<'a>(
        source_account: AccountInfo<'a>,
        destination_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = anchor_lang::system_program::Transfer {
            from: source_account,
            to: destination_account,
        };
        let cpi_context = anchor_lang::context::CpiContext::new(system_program, cpi_accounts);

        anchor_lang::system_program::transfer(cpi_context, amount)
    }

    pub fn realloc<'a>(
        funding_account: AccountInfo<'a>,
        target_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        new_len: usize,
        zero_init: bool,
    ) -> Result<()> {
        let new_minimum_balance = Rent::get()?.minimum_balance(new_len);
        let lamports_diff = new_minimum_balance.saturating_sub(target_account.try_lamports()?);

        Cortex::transfer_sol(
            funding_account,
            target_account.clone(),
            system_program,
            lamports_diff,
        )?;

        target_account
            .realloc(new_len, zero_init)
            .map_err(|_| ProgramError::InvalidRealloc.into())
    }
}

/// ------------------------------------------------------
/// Internal CPI Helpers
/// ------------------------------------------------------
impl Cortex {
    // recursive swap CPI
    #[allow(clippy::too_many_arguments)]
    pub fn internal_swap<'a>(
        &self,
        owner: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        funding_account: AccountInfo<'a>,
        receiving_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        receiving_custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        receiving_custody_token_account: AccountInfo<'a>,
        dispensing_custody: AccountInfo<'a>,
        dispensing_custody_token_account: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: SwapParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::Swap {
            caller: transfer_authority.clone(),
            owner,
            funding_account,
            receiving_account,
            transfer_authority: transfer_authority.clone(),
            cortex,
            pool,
            receiving_custody,
            oracle,
            receiving_custody_token_account,
            dispensing_custody,
            dispensing_custody_token_account,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;
        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::swap(cpi_context, params)
    }

    // recursive CPI
    #[allow(clippy::too_many_arguments)]
    pub fn internal_get_entry_price_and_fee<'a>(
        &self,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        collateral_custody: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: GetEntryPriceAndFeeParams,
    ) -> Result<NewPositionPricesAndFee> {
        let cpi_accounts = crate::cpi::accounts::GetEntryPriceAndFee {
            cortex,
            pool,
            custody,
            oracle,
            collateral_custody,
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts);

        let ret = crate::cpi::get_entry_price_and_fee(cpi_context, params)?;
        Ok(ret.get())
    }

    // recursive CPI
    #[allow(clippy::too_many_arguments)]
    pub fn internal_get_swap_amount_and_fee<'a>(
        &self,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        receiving_custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        dispensing_custody: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: GetSwapAmountAndFeesParams,
    ) -> Result<SwapAmountAndFees> {
        let cpi_accounts = crate::cpi::accounts::GetSwapAmountAndFees {
            cortex,
            pool,
            receiving_custody,
            oracle,
            dispensing_custody,
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts);

        let ret = crate::cpi::get_swap_amount_and_fees(cpi_context, params)?;
        Ok(ret.get())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_open_position_long<'a>(
        &self,
        caller: AccountInfo<'a>,
        owner: AccountInfo<'a>,
        payer: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        funding_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        custody_token_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: OpenPositionLongParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::OpenPositionLong {
            owner: owner.clone(),
            caller,
            payer,
            funding_account,
            transfer_authority,
            cortex,
            pool,
            position,
            custody,
            oracle,
            custody_token_account,
            system_program,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::open_position_long(cpi_context, params)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_open_position_short<'a>(
        &self,
        caller: AccountInfo<'a>,
        owner: AccountInfo<'a>,
        payer: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        funding_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        collateral_custody: AccountInfo<'a>,
        collateral_custody_token_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: OpenPositionShortParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::OpenPositionShort {
            owner: owner.clone(),
            caller,
            payer,
            funding_account,
            transfer_authority,
            cortex,
            pool,
            position,
            custody,
            oracle,
            collateral_custody,
            collateral_custody_token_account,
            system_program,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::open_position_short(cpi_context, params)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_close_position_long<'a>(
        &self,
        owner: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        receiving_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        custody_token_account: AccountInfo<'a>,
        user_profile: Option<AccountInfo<'a>>,
        referrer_profile: Option<AccountInfo<'a>>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: ClosePositionLongParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::ClosePositionLong {
            caller: transfer_authority.clone(),
            owner,
            receiving_account,
            transfer_authority: transfer_authority.clone(),
            cortex,
            pool,
            position,
            custody,
            oracle,
            custody_token_account,
            user_profile,
            referrer_profile,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;
        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::close_position_long(cpi_context, params)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_close_position_short<'a>(
        &self,
        owner: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        receiving_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        collateral_custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        collateral_custody_token_account: AccountInfo<'a>,
        user_profile: Option<AccountInfo<'a>>,
        referrer_profile: Option<AccountInfo<'a>>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: ClosePositionShortParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::ClosePositionShort {
            caller: transfer_authority.clone(),
            owner,
            receiving_account,
            transfer_authority: transfer_authority.clone(),
            cortex,
            pool,
            position,
            custody,
            oracle,
            collateral_custody,
            collateral_custody_token_account,
            user_profile,
            referrer_profile,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;
        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::close_position_short(cpi_context, params)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_increase_position_long<'a>(
        &self,
        caller: AccountInfo<'a>,
        owner: AccountInfo<'a>,
        payer: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        funding_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        custody_token_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: IncreasePositionLongParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::IncreasePositionLong {
            caller: caller.clone(),
            owner: owner.clone(),
            payer: payer.clone(),
            funding_account,
            transfer_authority,
            cortex,
            pool,
            position,
            custody,
            oracle,
            custody_token_account,
            system_program,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::increase_position_long(cpi_context, params)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn internal_increase_position_short<'a>(
        &self,
        caller: AccountInfo<'a>,
        owner: AccountInfo<'a>,
        payer: AccountInfo<'a>,
        transfer_authority: AccountInfo<'a>,
        funding_account: AccountInfo<'a>,
        cortex: AccountInfo<'a>,
        pool: AccountInfo<'a>,
        position: AccountInfo<'a>,
        custody: AccountInfo<'a>,
        oracle: AccountInfo<'a>,
        collateral_custody: AccountInfo<'a>,
        collateral_custody_token_account: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        adrena_program: AccountInfo<'a>,
        params: IncreasePositionShortParams,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let cpi_accounts = crate::cpi::accounts::IncreasePositionShort {
            caller: caller.clone(),
            owner: owner.clone(),
            payer: payer.clone(),
            funding_account,
            transfer_authority,
            cortex,
            pool,
            position,
            custody,
            oracle,
            collateral_custody,
            collateral_custody_token_account,
            system_program,
            token_program,
            adrena_program: adrena_program.clone(),
        };

        let cpi_program = adrena_program;

        let cpi_context = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts)
            .with_signer(authority_seeds);

        crate::cpi::increase_position_short(cpi_context, params)
    }

    pub fn create_pda_account<'info>(
        payer: &Signer<'info>,
        account: &AccountInfo<'info>,
        system_program: &Program<'info, System>,
        data_len: usize,
        initialize_fn: impl FnOnce(&mut [u8]) -> Result<()>,
        seeds: &[&[u8]],
    ) -> Result<()> {
        let required_lamports = Rent::get()?.minimum_balance(data_len);

        if account.lamports() == 0 {
            // If the account doesn't exist, use the create instruction (most effective way CU-wise)
            invoke_signed(
                &system_instruction::create_account(
                    &payer.key(),
                    &account.key(),
                    required_lamports,
                    data_len as u64,
                    &Adrena::id(),
                ),
                &[
                    payer.to_account_info().clone(),
                    account.clone(),
                    system_program.to_account_info().clone(),
                ],
                &[seeds],
            )?;
        } else {
            // The account has lamports, need multiple steps to make sure everything is set properly

            // Check if the account has enough lamports, otherwise transfer the difference
            let current_lamports = account.lamports();
            if current_lamports < required_lamports {
                invoke_signed(
                    &system_instruction::transfer(
                        &payer.key(),
                        &account.key(),
                        required_lamports - current_lamports,
                    ),
                    &[
                        payer.to_account_info(),
                        account.to_account_info(),
                        system_program.to_account_info(),
                    ],
                    &[seeds],
                )?;
            }

            // Allocate space for the account
            invoke_signed(
                &system_instruction::allocate(&account.key(), data_len as u64),
                &[account.to_account_info(), system_program.to_account_info()],
                &[seeds],
            )?;

            // Assign the account to the correct program
            invoke_signed(
                &system_instruction::assign(&account.key(), &Adrena::id()),
                &[account.to_account_info(), system_program.to_account_info()],
                &[seeds],
            )?;
        }

        // Initialize the account's data
        let data: &mut [u8] = &mut account.data.borrow_mut();
        initialize_fn(data)?;

        // Save the account
        account.exit(&Adrena::id()).unwrap();

        Ok(())
    }
}
