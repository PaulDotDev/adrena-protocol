use {
    crate::{error::AdrenaError, state::cortex::Cortex},
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitReferrerRewardTokenVault<'info> {
    /// #1
    /// Anyone
    pub caller: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, will be set as authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState,
        has_one = fee_redistribution_mint,
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        init,
        payer = payer,
        token::authority = transfer_authority,
        token::mint = fee_redistribution_mint,
        seeds = [b"referrer_reward_token_vault", cortex.load()?.fee_redistribution_mint.as_ref()],
        bump
    )]
    pub referrer_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #6
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #7
    pub system_program: Program<'info, System>,

    /// #8
    pub token_program: Program<'info, Token>,

    /// #9
    pub rent: Sysvar<'info, Rent>,
}

pub fn init_referrer_reward_token_vault(_: Context<InitReferrerRewardTokenVault>) -> Result<()> {
    msg!("Init referrer reward token vault");
    Ok(())
}
