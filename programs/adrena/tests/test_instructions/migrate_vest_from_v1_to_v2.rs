use {
    crate::utils::{self, pda},
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn migrate_vest_from_v1_to_v2(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    payer: &Keypair,
    owner: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let vest_pda = pda::get_vest_pda(&owner.pubkey()).0;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::MigrateVestFromV1ToV2 {
            caller: caller.pubkey(),
            owner: owner.pubkey(),
            payer: payer.pubkey(),
            vest: vest_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::MigrateVestFromV1ToV2 {},
        Some(&payer.pubkey()),
        &[payer, caller],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================

    Ok(())
}
