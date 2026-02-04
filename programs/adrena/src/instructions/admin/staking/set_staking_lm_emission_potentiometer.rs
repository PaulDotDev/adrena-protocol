use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, staking::Staking},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetStakingLmEmissionPotentiometers<'info> {
    /// #1
    #[account()]
    pub admin: Signer<'info>,

    /// #2
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #3
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SetStakingLmEmissionPotentiometersParams {
    pub lm_emission_potentiometer_bps: u16,
}

pub fn set_staking_lm_emission_potentiometers<'info>(
    ctx: Context<'_, '_, '_, 'info, SetStakingLmEmissionPotentiometers<'info>>,
    params: &SetStakingLmEmissionPotentiometersParams,
) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;

    if !staking
        .validate_lm_emission_potentiometer_bps_in_range(params.lm_emission_potentiometer_bps)
    {
        return Err(ProgramError::InvalidArgument.into());
    }

    // update Staking data
    {
        staking.lm_emission_potentiometer_bps = params.lm_emission_potentiometer_bps;
    }

    Ok(())
}
