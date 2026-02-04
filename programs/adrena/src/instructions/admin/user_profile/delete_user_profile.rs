use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, user_profile::UserProfile},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct DeleteUserProfile<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    /// CHECK: Can be any wallet
    pub user: AccountInfo<'info>,

    /// #3
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"user_profile",
                 user.key().as_ref()],
        bump = user_profile.load()?.bump,
        close = payer
    )]
    pub user_profile: AccountLoader<'info, UserProfile>,

    /// #5
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #6
    system_program: Program<'info, System>,
}

pub fn delete_user_profile(ctx: Context<DeleteUserProfile>) -> Result<()> {
    ctx.accounts.cortex.load_mut()?.user_profiles_count -= 1;

    Ok(())
}
