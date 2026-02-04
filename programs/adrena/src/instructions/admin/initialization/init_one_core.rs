use {
    crate::state::cortex::{Cortex, CortexInitializationStep},
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct InitOne<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, will be set as authority for token accounts
    #[account(
        init,
        payer = payer,
        space = 0,
        seeds = [b"transfer_authority"],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        init,
        payer = payer,
        space = Cortex::LEN,
        seeds = [b"cortex"],
        bump
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        constraint = protocol_fee_recipient.mint == fee_redistribution_mint.key(),
    )]
    pub protocol_fee_recipient: Box<Account<'info, TokenAccount>>,

    /// #6
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #7
    #[account(
        init,
        payer = payer,
        mint::authority = transfer_authority,
        mint::freeze_authority = transfer_authority,
        mint::decimals = Cortex::LM_DECIMALS,
        seeds = [b"lm_token_mint"],
        bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,

    /// #10
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct InitOneParams {
    pub core_contributor_bucket_allocation: u64,
    pub foundation_bucket_allocation: u64,
    pub ecosystem_bucket_allocation: u64,
}

pub fn init_one_core<'info>(
    ctx: Context<'_, '_, '_, 'info, InitOne<'info>>,
    params: &InitOneParams,
) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_init()?;

    cortex.admin = ctx.accounts.admin.key();
    cortex.initialized = CortexInitializationStep::Step1.into();

    // Fee redistribution mint
    {
        cortex.protocol_fee_recipient = ctx.accounts.protocol_fee_recipient.key();
        cortex.fee_conversion_decimals = ctx.accounts.fee_redistribution_mint.decimals;
        cortex.fee_redistribution_mint = ctx.accounts.fee_redistribution_mint.key();
    }

    // Lm tokens minting rules
    {
        cortex.core_contributor_bucket_allocation = params.core_contributor_bucket_allocation;
        cortex.core_contributor_bucket_vested_amount = u64::MIN;
        cortex.core_contributor_bucket_minted_amount = u64::MIN;

        cortex.foundation_bucket_allocation = params.foundation_bucket_allocation;
        cortex.foundation_bucket_vested_amount = u64::MIN;
        cortex.foundation_bucket_minted_amount = u64::MIN;

        cortex.ecosystem_bucket_allocation = params.ecosystem_bucket_allocation;
        cortex.ecosystem_bucket_vested_amount = u64::MIN;
        cortex.ecosystem_bucket_minted_amount = u64::MIN;
    }

    // Bumps
    {
        cortex.lm_token_bump = ctx.bumps.lm_token_mint;
        cortex.lm_token_mint = ctx.accounts.lm_token_mint.key();
        cortex.bump = ctx.bumps.cortex;
        cortex.transfer_authority_bump = ctx.bumps.transfer_authority;
    }

    Ok(())
}
