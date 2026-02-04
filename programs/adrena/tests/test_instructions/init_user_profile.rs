use {
    crate::utils::{self, pda},
    adrena::{instructions::InitUserProfileParams, state::user_profile::UserProfile},
    anchor_lang::ToAccountMetas,
    solana_program::pubkey::Pubkey,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_user_profile(
    program_test_ctx: &RwLock<ProgramTestContext>,
    user: Pubkey,
    caller: &Keypair,
    payer: &Keypair,
    params: InitUserProfileParams,
    referrer_profile: Option<Pubkey>,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== GIVEN =============================================================
    let cortex_pda = pda::get_cortex_pda().0;
    let (user_profile_pda, user_profile_bump) = pda::get_user_profile_pda(&user);
    let user_nickname_pda = pda::get_user_profile_nickname_pda(params.nickname.clone()).0;

    // ==== WHEN ==============================================================

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitUserProfile {
            user,
            caller: caller.pubkey(),
            payer: payer.pubkey(),
            user_profile: user_profile_pda,
            cortex: cortex_pda,
            system_program: anchor_lang::system_program::ID,
            user_nickname: user_nickname_pda,
            referrer_profile,
        }
        .to_account_metas(None),
        adrena::instruction::InitUserProfile {
            params: InitUserProfileParams {
                nickname: params.nickname.clone(),
                profile_picture: params.profile_picture,
                wallpaper: params.wallpaper,
                title: params.title,
                team: params.team,
                continent: params.continent,
            },
        },
        Some(&payer.pubkey()),
        &[caller, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_user_profile", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let user_profile_account =
        utils::get_zero_copy_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    {
        assert_eq!(user_profile_account.bump, user_profile_bump);
        assert_eq!(user_profile_account.nickname.to_string(), params.nickname);
        assert_eq!(user_profile_account.owner, user);
        assert!(user_profile_account.created_at > 0);
    }

    Ok((user_profile_pda, user_profile_bump))
}
