#[cfg(not(feature = "test"))]
use crate::error::AdrenaError;
use {
    crate::state::user_profile::{Achievement, UserProfile},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GrantOrRemoveAchievement<'info> {
    /// #1
    pub whitelisted_caller: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Can be any wallet
    pub user: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"user_profile",
            user.key().as_ref()],
        bump = user_profile.load()?.bump
    )]
    pub user_profile: AccountLoader<'info, UserProfile>,

    /// #5
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GrantOrRemoveAchievementParams {
    pub achievements: Vec<u8>,
    pub operation: u8,
}

pub fn grant_or_remove_achievement(
    ctx: Context<GrantOrRemoveAchievement>,
    params: GrantOrRemoveAchievementParams,
) -> Result<()> {
    #[cfg(not(feature = "test"))]
    {
        // Make sure only the program or a whitelisted authority can modify achievements
        // Dedicated only to achievements and owned by the team
        let allowed_whitelisted_caller: Pubkey =
            solana_program::pubkey!("4z2wedqmMft5gmt2h67pnRLp5p4v34CH7eJ14Hy74FqE");

        require!(
            ctx.accounts.whitelisted_caller.key() == allowed_whitelisted_caller,
            AdrenaError::InvalidCaller
        );
    }

    let mut user_profile = ctx.accounts.user_profile.load_mut()?;

    match params.operation {
        // Unlock
        1 => {
            for &achievement_id in params.achievements.iter() {
                let achievement = Achievement::try_from(achievement_id).unwrap();

                user_profile.unlock_achievement(achievement);
            }
        }
        // Remove
        2 => {
            for &achievement_id in params.achievements.iter() {
                let achievement = Achievement::try_from(achievement_id).unwrap();

                user_profile.remove_achievement(achievement);
            }
        }
        _ => return Err(ProgramError::InvalidArgument.into()),
    }

    Ok(())
}
