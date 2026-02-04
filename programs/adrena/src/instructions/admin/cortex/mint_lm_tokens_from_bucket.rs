use {
    crate::{error::AdrenaError, state::cortex::Cortex},
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct MintLmTokensFromBucket<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = receiving_account.mint == lm_token_mint.key(),
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        // Checked in the instruction as it also accepts the transfer authority on this call
        // has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #6
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintLmTokensFromBucketParams {
    pub bucket_name: u8, // BucketName
    pub amount: u64,
    pub reason: String,
}

#[derive(PartialEq, Copy, Clone, Default, Debug)]
pub enum BucketName {
    CoreContributor = 0,
    Foundation = 1,
    #[default]
    Ecosystem = 2,
}

// implement from instead
impl From<BucketName> for u8 {
    fn from(bucket_name: BucketName) -> u8 {
        match bucket_name {
            BucketName::CoreContributor => 0,
            BucketName::Foundation => 1,
            BucketName::Ecosystem => 2,
        }
    }
}

impl TryFrom<u8> for BucketName {
    type Error = error::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => BucketName::CoreContributor,
            1 => BucketName::Foundation,
            2 => BucketName::Ecosystem,
            // Return an error if unknown value
            _ => Err(AdrenaError::InvalidBucketName)?,
        })
    }
}

pub fn mint_lm_tokens_from_bucket<'info>(
    ctx: Context<'_, '_, '_, 'info, MintLmTokensFromBucket<'info>>,
    params: &MintLmTokensFromBucketParams,
) -> Result<u8> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    // Preliminary checks
    {
        if params.amount == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        // Check authentication, accept either transfer_authority (internal call) as admin or the admin (external call)
        if ctx.accounts.admin.key() == ctx.accounts.transfer_authority.key() {
            // Good
        } else {
            require_eq!(cortex.admin, ctx.accounts.admin.key(),);
        }
    }

    let bucket_name = BucketName::try_from(params.bucket_name)?;

    msg!(
        "Mint {} LM tokens for {} bucket",
        params.amount,
        match bucket_name {
            BucketName::CoreContributor => "core_contributor",
            BucketName::Foundation => "foundation",
            BucketName::Ecosystem => "ecosystem",
        }
    );

    msg!("Reason: {}", params.reason);

    cortex.update_bucket_minted_amount(bucket_name, params.amount)?;

    {
        let a = cortex.core_contributor_bucket_minted_amount;
        let b = cortex.core_contributor_bucket_allocation;
        msg!("Core contributor bucket: {}/{}", a, b);
    }

    {
        let a = cortex.foundation_bucket_minted_amount;
        let b = cortex.foundation_bucket_allocation;
        msg!("Foundation bucket: {}/{}", a, b);
    }

    {
        let a = cortex.ecosystem_bucket_minted_amount;
        let b = cortex.ecosystem_bucket_allocation;
        msg!("Ecosystem bucket: {}/{}", a, b);
    }

    cortex.mint_tokens(
        ctx.accounts.lm_token_mint.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    Ok(0)
}
