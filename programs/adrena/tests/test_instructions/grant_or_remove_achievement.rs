use {
    crate::utils::pda,
    adrena::instructions::public::user_profile_p::grant_or_remove_achievement::GrantOrRemoveAchievementParams,
    anchor_lang::{prelude::*, InstructionData},
    solana_program_test::*,
    solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
        transaction::Transaction,
    },
    tokio::sync::RwLock,
};

pub fn get_grant_or_remove_achievement_ix(
    whitelisted_caller: &Keypair,
    payer: &Keypair,
    user: &Pubkey,
    params: GrantOrRemoveAchievementParams,
) -> Instruction {
    let user_profile_pda = pda::get_user_profile_pda(user).0;

    let accounts = adrena::accounts::GrantOrRemoveAchievement {
        whitelisted_caller: whitelisted_caller.pubkey(),
        payer: payer.pubkey(),
        user: *user,
        user_profile: user_profile_pda,
        system_program: anchor_lang::system_program::ID,
    };

    Instruction {
        program_id: adrena::id(),
        accounts: accounts.to_account_metas(None),
        data: adrena::instruction::GrantOrRemoveAchievement { params }.data(),
    }
}

pub async fn grant_or_remove_achievement(
    program_test_ctx: &RwLock<ProgramTestContext>,
    whitelisted_caller: &Keypair,
    payer: &Keypair,
    user: &Pubkey,
    params: GrantOrRemoveAchievementParams,
) -> std::result::Result<(), BanksClientError> {
    let ix = get_grant_or_remove_achievement_ix(whitelisted_caller, payer, user, params);

    let mut ctx = program_test_ctx.write().await;
    let recent_blockhash = ctx.last_blockhash;

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[whitelisted_caller, payer],
        recent_blockhash,
    );

    println!("transaction: {:?}", transaction);

    ctx.banks_client.process_transaction(transaction).await
}
