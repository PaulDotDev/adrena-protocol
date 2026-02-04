use {
    crate::utils::{self, pda},
    adrena::state::cortex::Cortex,
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn claim_referral_fee(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    referrer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let referrer_profile_pda = pda::get_user_profile_pda(&referrer.pubkey()).0;
    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let referrer_reward_token_vault_pda =
        pda::get_referrer_reward_token_vault(&cortex_account.fee_redistribution_mint).0;

    let receiving_account = utils::find_associated_token_account(
        &referrer.pubkey(),
        &cortex_account.fee_redistribution_mint,
    )
    .0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ClaimReferralFee {
            referrer: referrer.pubkey(),
            referrer_profile: referrer_profile_pda,
            referrer_reward_token_vault: referrer_reward_token_vault_pda,
            receiving_account,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::ClaimReferralFee {},
        Some(&payer.pubkey()),
        &[payer, referrer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("claim_referral_fee", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}
