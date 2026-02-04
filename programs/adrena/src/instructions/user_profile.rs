pub fn change_title(ctx: Context<ChangeUserProfileSetting>, title: Title) -> Result<()> {
    // Verify the user has unlocked this title
    if !ctx.accounts.user_profile.can_use_title(&title) {
        return err!(AdrenaError::TitleNotAvailable);
    }

    ctx.accounts.user_profile.selected_title = title;
    Ok(())
}

pub fn change_profile_picture(
    ctx: Context<ChangeUserProfileSetting>,
    profile_picture: ProfilePicture,
) -> Result<()> {
    // Verify the user has unlocked this profile picture
    if !ctx.accounts.user_profile.can_use_pfp(&profile_picture) {
        return err!(AdrenaError::ProfilePictureNotAvailable);
    }

    ctx.accounts.user_profile.selected_pfp = profile_picture;
    Ok(())
}

pub fn change_wallpaper(
    ctx: Context<ChangeUserProfileSetting>,
    wallpaper: Wallpaper,
) -> Result<()> {
    // Verify the user has unlocked this wallpaper
    if !ctx.accounts.user_profile.can_use_wallpaper(&wallpaper) {
        return err!(AdrenaError::WallpaperNotAvailable);
    }

    ctx.accounts.user_profile.selected_wallpaper = wallpaper;
    Ok(())
}
