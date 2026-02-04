use {
    crate::{error::AdrenaError, state::cortex::Cortex},
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, TokenAccount},
};

#[derive(Accounts)]
pub struct SetProtocolFeeRecipient<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        has_one = fee_redistribution_mint,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #3
    #[account(
        constraint = protocol_fee_recipient.mint == fee_redistribution_mint.key(),
    )]
    pub protocol_fee_recipient: Box<Account<'info, TokenAccount>>,

    /// #4
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,
}

pub fn set_protocol_fee_recipient(ctx: Context<SetProtocolFeeRecipient>) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.protocol_fee_recipient = ctx.accounts.protocol_fee_recipient.key();

    msg!(
        "Protocol fee recipient is now: {}",
        cortex.protocol_fee_recipient.to_string()
    );

    Ok(())
}
