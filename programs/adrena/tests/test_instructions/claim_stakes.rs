use {
    crate::utils::{self, pda},
    adrena::state::cortex::Cortex,
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn claim_stakes(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    payer: &Keypair,
    owner: &Pubkey,
    pool_pda: &Pubkey,
    staked_token_mint: &Pubkey,
    params: &adrena::instructions::ClaimStakesParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&staking_pda).0;

    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let reward_token_account_address =
        utils::find_associated_token_account(owner, &cortex_account.fee_redistribution_mint).0;

    let lm_token_account_address =
        utils::find_associated_token_account(owner, &lm_token_mint_pda).0;

    // ==== WHEN ==============================================================

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ClaimStakes {
            caller: caller.pubkey(),
            owner: *owner,
            payer: payer.pubkey(),
            reward_token_account: reward_token_account_address,
            lm_token_account: lm_token_account_address,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            transfer_authority: transfer_authority_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            genesis_lock: genesis_lock_pda,
            lm_token_mint: lm_token_mint_pda,
            fee_redistribution_mint: cortex_account.fee_redistribution_mint,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::ClaimStakes {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[caller, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("claim_stakes", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}
