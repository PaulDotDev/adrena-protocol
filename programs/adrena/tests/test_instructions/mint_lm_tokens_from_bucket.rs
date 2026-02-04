use {
    crate::utils::{self, pda},
    adrena::instructions::MintLmTokensFromBucketParams,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn mint_lm_tokens_from_bucket(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    owner: &Pubkey,
    payer: &Keypair,
    params: MintLmTokensFromBucketParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lm_token_account_address =
        utils::find_associated_token_account(owner, &lm_token_mint_pda).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::MintLmTokensFromBucket {
            admin: admin.pubkey(),
            receiving_account: lm_token_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            lm_token_mint: lm_token_mint_pda,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::MintLmTokensFromBucket {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("mint_lm_tokens_from_bucket", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}
