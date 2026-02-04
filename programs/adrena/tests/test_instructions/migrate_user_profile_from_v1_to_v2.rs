use {
    crate::utils::{self, pda},
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn get_migrate_user_profile_from_v1_to_v2_ix(
    payer: &Keypair,
    caller: &Keypair,
    owner: Pubkey,
    nickname: &str,
) -> std::result::Result<solana_sdk::instruction::Instruction, BanksClientError> {
    let ix = solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: get_migrate_user_profile_from_v1_to_v2_accounts_meta(
            payer, caller, owner, nickname,
        )
        .await,
        data: adrena::instruction::MigrateUserProfileFromV1ToV2 {
            params: adrena::instructions::MigrateUserProfileFromV1ToV2Params {
                nickname: nickname.to_string(),
            },
        }
        .data(),
    };

    Ok(ix)
}

pub async fn get_migrate_user_profile_from_v1_to_v2_accounts_meta(
    payer: &Keypair,
    caller: &Keypair,
    owner: Pubkey,
    nickname: &str,
) -> Vec<anchor_lang::prelude::AccountMeta> {
    let user_profile_pda = pda::get_user_profile_pda(&owner).0;
    let user_nickname_pda = pda::get_user_profile_nickname_pda(nickname.to_string()).0;

    adrena::accounts::MigrateUserProfileFromV1ToV2 {
        owner,
        caller: caller.pubkey(),
        payer: payer.pubkey(),
        user_profile: user_profile_pda,
        user_nickname: user_nickname_pda,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        rent: solana_program::sysvar::rent::ID,
    }
    .to_account_metas(None)
}

pub async fn migrate_user_profile_from_v1_to_v2(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    caller: &Keypair,
    owner: Pubkey,
    nickname: &str,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        get_migrate_user_profile_from_v1_to_v2_accounts_meta(payer, caller, owner, nickname).await,
        adrena::instruction::MigrateUserProfileFromV1ToV2 {
            params: adrena::instructions::MigrateUserProfileFromV1ToV2Params {
                nickname: nickname.to_string(),
            },
        },
        Some(&payer.pubkey()),
        &[payer, caller],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================

    Ok(())
}
