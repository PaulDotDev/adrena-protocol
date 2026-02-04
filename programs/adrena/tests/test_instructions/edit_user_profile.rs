use {
    crate::utils::{self, pda},
    adrena::{instructions::EditUserProfileParams, state::user_profile::UserProfile},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn edit_user_profile(
    program_test_ctx: &RwLock<ProgramTestContext>,
    user: &Keypair,
    payer: &Keypair,
    params: EditUserProfileParams,
    referrer_profile: Option<Pubkey>,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let user_profile_pda = pda::get_user_profile_pda(&user.pubkey()).0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::EditUserProfile {
            user: user.pubkey(),
            payer: payer.pubkey(),
            user_profile: user_profile_pda,
            referrer_profile,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        adrena::instruction::EditUserProfile {
            params: EditUserProfileParams {
                profile_picture: params.profile_picture,
                wallpaper: params.wallpaper,
                title: params.title,
                team: params.team,
                continent: params.continent,
            },
        },
        Some(&payer.pubkey()),
        &[user, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("edit_user_profile", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let user_profile_account =
        utils::get_zero_copy_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    {
        assert_eq!(user_profile_account.profile_picture, params.profile_picture);
        assert_eq!(user_profile_account.wallpaper, params.wallpaper);
    }

    Ok(())
}
