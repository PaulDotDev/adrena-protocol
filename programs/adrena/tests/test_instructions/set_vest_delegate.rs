use {
    crate::utils::{self, pda},
    adrena::instructions::SetVestDelegateParams,
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_vest_delegate(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    payer: &Keypair,
    owner: &Keypair,
    params: &SetVestDelegateParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let vest_pda = pda::get_vest_pda(&owner.pubkey()).0;
    let cortex_pda = pda::get_cortex_pda().0;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetVestDelegate {
            caller: caller.pubkey(),
            owner: owner.pubkey(),
            payer: payer.pubkey(),
            cortex: cortex_pda,
            vest: vest_pda,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        adrena::instruction::SetVestDelegate { params: *params },
        Some(&payer.pubkey()),
        &[payer, caller],
        None,
        None,
    )
    .await?;

    // ==== THEN ==============================================================

    Ok(())
}
