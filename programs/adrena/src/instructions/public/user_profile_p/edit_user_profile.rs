use {
    crate::{
        error::AdrenaError,
        state::user_profile::{Continent, ProfilePicture, Team, Title, UserProfile, Wallpaper},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct EditUserProfile<'info> {
    /// #1
    pub user: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        seeds = [b"user_profile",
                 user.key().as_ref()],
        bump = user_profile.load()?.bump
    )]
    pub user_profile: AccountLoader<'info, UserProfile>,

    /// #4
    /// Apply this referrer to the user profile, If none, referrer_profile is set to default
    pub referrer_profile: Option<AccountLoader<'info, UserProfile>>,

    /// #5
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct EditUserProfileParams {
    pub profile_picture: u8,
    pub wallpaper: u8,
    pub title: u8,
    pub team: Option<u8>,
    pub continent: Option<u8>,
}

pub fn edit_user_profile(
    ctx: Context<EditUserProfile>,
    params: &EditUserProfileParams,
) -> Result<()> {
    // Preliminary checks
    {
        // Validate that the parameters are valid enum values
        let wallpaper = match Wallpaper::try_from(params.wallpaper) {
            Ok(w) => w,
            Err(_) => return Err(AdrenaError::InvalidWallpaperOrProfilePictureOrTitle.into()),
        };

        let profile_picture = match ProfilePicture::try_from(params.profile_picture) {
            Ok(p) => p,
            Err(_) => return Err(AdrenaError::InvalidWallpaperOrProfilePictureOrTitle.into()),
        };

        let title = match Title::try_from(params.title) {
            Ok(t) => t,
            Err(_) => return Err(AdrenaError::InvalidWallpaperOrProfilePictureOrTitle.into()),
        };

        let team = if let Some(param_team) = params.team {
            match Team::try_from(param_team) {
                Ok(t) => Some(t),
                Err(_) => return Err(AdrenaError::InvalidContinentOrTeam.into()),
            }
        } else {
            None
        };

        let continent = if let Some(param_continent) = params.continent {
            match Continent::try_from(param_continent) {
                Ok(c) => Some(c),
                Err(_) => return Err(AdrenaError::InvalidContinentOrTeam.into()),
            }
        } else {
            None
        };

        // Check if the user has unlocked these items through achievements
        let user_profile = ctx.accounts.user_profile.load()?;

        require!(
            user_profile.can_use_wallpaper(wallpaper),
            AdrenaError::WallpaperNotUnlocked
        );

        require!(
            user_profile.can_use_pfp(profile_picture),
            AdrenaError::ProfilePictureNotUnlocked
        );

        require!(
            user_profile.can_use_title(title),
            AdrenaError::TitleNotUnlocked
        );

        // Handle team changes if provided
        if let Some(param_team) = team {
            // Cannot reset team to default
            if param_team == Team::Default {
                return Err(AdrenaError::InvalidContinentOrTeam.into());
            }

            // Cannot change team after it's set
            if user_profile.get_team() != Team::Default {
                return Err(AdrenaError::TeamImmutable.into());
            }
        }

        // Handle continent changes if provided
        if let Some(param_continent) = continent {
            // Cannot reset continent to default
            if param_continent == Continent::Default {
                return Err(AdrenaError::InvalidContinentOrTeam.into());
            }
        }

        // Cannot set yourself as a referrer
        if let Some(referrer_profile) = ctx.accounts.referrer_profile.as_ref() {
            require!(
                referrer_profile.key().ne(&ctx.accounts.user_profile.key()),
                AdrenaError::InvalidAccountData,
            );
        }
    }

    {
        let mut user_profile = ctx.accounts.user_profile.load_mut()?;

        user_profile.wallpaper = params.wallpaper;
        user_profile.profile_picture = params.profile_picture;
        user_profile.title = params.title;

        if let Some(param_team) = params.team {
            user_profile.team = param_team;
        }

        if let Some(param_continent) = params.continent {
            user_profile.continent = param_continent;
        }

        if let Some(referrer_profile) = ctx.accounts.referrer_profile.as_ref() {
            user_profile.referrer_profile = referrer_profile.key();
        } else {
            user_profile.referrer_profile = Pubkey::default();
        }
    }

    Ok(())
}
