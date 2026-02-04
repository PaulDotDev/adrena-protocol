use {
    crate::utils::{self, pda},
    adrena::instructions::MintStakedLmTokensFromBucketParams,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn mint_staked_lm_tokens_from_bucket(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    owner: &Pubkey,
    payer: &Keypair,
    params: MintStakedLmTokensFromBucketParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;
    let staking_staked_token_vault_pda = pda::get_staking_staked_token_vault_pda(&staking_pda).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::MintStakedLmTokensFromBucket {
            admin: admin.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            lm_token_mint: lm_token_mint_pda,
            payer: payer.pubkey(),
            owner: *owner,
            staking_staked_token_vault: staking_staked_token_vault_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        adrena::instruction::MintStakedLmTokensFromBucket {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction(
        "mint_staked_lm_tokens_from_bucket",
        "admin",
        ix_size,
        tx_used_cu,
    );

    // ==== THEN ==============================================================

    Ok(())
}
