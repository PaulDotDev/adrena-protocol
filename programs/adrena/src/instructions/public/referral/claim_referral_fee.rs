use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, user_profile::UserProfile},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct ClaimReferralFee<'info> {
    /// #1
    #[account(mut)]
    pub referrer: Signer<'info>,

    /// #2
    #[account(
        mut,
        constraint = receiving_account.mint == cortex.load()?.fee_redistribution_mint,
        constraint = receiving_account.owner == referrer.key(),
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: UncheckedAccount<'info>,

    /// #4
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        mut,
        seeds = [b"user_profile",
            referrer.key().as_ref()],
        bump = referrer_profile.load()?.bump
    )]
    pub referrer_profile: AccountLoader<'info, UserProfile>,

    /// #6
    #[account(
        mut,
        seeds = [b"referrer_reward_token_vault", cortex.load()?.fee_redistribution_mint.as_ref()],
        bump
    )]
    pub referrer_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #7
    pub system_program: Program<'info, System>,

    /// #8
    pub token_program: Program<'info, Token>,
}

// It has been assumed during fee distribution to the vault that the redistribution mint is a stable and that the rate of exchange with usd is 1:1
pub fn claim_referral_fee(ctx: Context<ClaimReferralFee>) -> Result<()> {
    let mut referrer_profile = ctx.accounts.referrer_profile.load_mut()?;
    let cortex = ctx.accounts.cortex.load()?;

    if referrer_profile.claimable_referral_fee_usd == 0 {
        msg!("No referral fee to claim");
        return Ok(());
    }

    msg!(
        "Referral fee to claim in usd: {}",
        referrer_profile.claimable_referral_fee_usd
    );

    // Assume that the redistribution mint is a stable coin and that the rate of exchange with usd is 1:1
    let claimable_amount = referrer_profile.claimable_referral_fee_usd;

    // Pay referrer profits from the referrer vault
    cortex.transfer_tokens(
        ctx.accounts.referrer_reward_token_vault.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        claimable_amount,
    )?;

    referrer_profile.claimable_referral_fee_usd = 0;

    Ok(())
}
