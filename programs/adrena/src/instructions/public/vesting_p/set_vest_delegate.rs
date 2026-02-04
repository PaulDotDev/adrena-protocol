use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, vest::Vest},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
#[instruction()]
pub struct SetVestDelegate<'info> {
    /// #1
    #[account(
        // Only the owner of the vest, or the admin can call this instruction
        constraint = caller.key() == vest.load()?.owner || caller.key() == cortex.load()?.admin,
    )]
    pub caller: Signer<'info>,

    /// #2
    /// CHECK: Checked to be the wallet related to the vest
    pub owner: AccountInfo<'info>,

    /// #3
    /// CHECK: Any account
    #[account(mut)]
    pub payer: AccountInfo<'info>,

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
        seeds = [b"vest", owner.key().as_ref()],
        bump = vest.load()?.bump,
        has_one = owner,
        constraint = !vest.load()?.is_cancelled() @AdrenaError::InvalidVestState,
        constraint = vest.load()?.version == Vest::VERSION @AdrenaError::InvalidVestVersion,
    )]
    pub vest: AccountLoader<'info, Vest>,

    /// #6
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct SetVestDelegateParams {
    pub delegate: Option<Pubkey>,
}

pub fn set_vest_delegate<'info>(
    ctx: Context<'_, '_, '_, 'info, SetVestDelegate<'info>>,
    params: &SetVestDelegateParams,
) -> Result<()> {
    let mut vest = ctx.accounts.vest.load_mut()?;

    vest.delegate = params.delegate.unwrap_or_default();
    vest.has_delegate = params.delegate.is_some() as u8;

    Ok(())
}
