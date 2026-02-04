use {
    crate::utils::{self, pda},
    adrena::{instructions::EditUserProfileNicknameParams, state::user_profile::UserProfile},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn edit_user_profile_nickname(
    program_test_ctx: &RwLock<ProgramTestContext>,
    user: &Keypair,
    payer: &Keypair,
    params: EditUserProfileNicknameParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let user_profile_pda = pda::get_user_profile_pda(&user.pubkey()).0;
    let cortex_pda: Pubkey = pda::get_cortex_pda().0;
    let lm_token_mint = utils::pda::get_lm_token_mint_pda().0;
    let user_nickname_pda = pda::get_user_profile_nickname_pda(params.nickname.clone()).0;

    let user_profile_account =
        utils::get_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    let old_user_nickname = if user_profile_account.nickname.length > 0 {
        Some(pda::get_user_profile_nickname_pda(user_profile_account.nickname.to_string()).0)
    } else {
        None
    };

    let funding_account_address =
        utils::find_associated_token_account(&user.pubkey(), &lm_token_mint).0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::EditUserProfileNickname {
            user_profile: user_profile_pda,
            system_program: anchor_lang::system_program::ID,
            owner: user.pubkey(),
            cortex: cortex_pda,
            funding_account: funding_account_address,
            lm_token_mint,
            old_user_nickname,
            user_nickname: user_nickname_pda,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::EditUserProfileNickname {
            params: EditUserProfileNicknameParams {
                nickname: params.nickname.clone(),
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
        assert_eq!(user_profile_account.nickname.to_string(), params.nickname);
    }

    Ok(())
}
