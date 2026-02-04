use {
    crate::{error::AdrenaError, state::cortex::Cortex},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetAdminParams {
    pub new_admin: Pubkey,
}

pub fn set_admin<'info>(
    ctx: Context<'_, '_, '_, 'info, SetAdmin<'info>>,
    params: &SetAdminParams,
) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.admin = params.new_admin;

    msg!("Admin is now: {}", cortex.admin.to_string());

    Ok(())
}
