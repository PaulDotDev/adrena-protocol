use {
    crate::utils::{self, pda},
    adrena::state::{cortex::Cortex, user_staking::UserStaking},
    anchor_lang::ToAccountMetas,
    solana_program::pubkey::Pubkey,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn init_user_staking(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    staked_token_mint: &Pubkey,
    pool_pda: &Pubkey,
) -> std::result::Result<(Pubkey, u8, usize, u64), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let (user_staking_pda, user_staking_bump) =
        pda::get_user_staking_pda(&owner.pubkey(), &staking_pda);
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&staking_pda).0;
    let reward_token_account_address = utils::find_associated_token_account(
        &owner.pubkey(),
        &cortex_account.fee_redistribution_mint,
    )
    .0;
    let lm_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lm_token_mint_pda).0;

    // ==== WHEN ==============================================================

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::InitUserStaking {
            caller: owner.pubkey(),
            payer: owner.pubkey(),
            owner: owner.pubkey(),
            reward_token_account: reward_token_account_address,
            lm_token_account: lm_token_account_address,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            lm_token_mint: lm_token_mint_pda,
            fee_redistribution_mint: cortex_account.fee_redistribution_mint,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::InitUserStaking {},
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("init_user_staking", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let user_staking_account =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    {
        assert_eq!(user_staking_account.bump, user_staking_bump);
        assert_eq!(user_staking_account.liquid_stake.amount, 0);
    }

    Ok((user_staking_pda, user_staking_bump, ix_size, tx_used_cu))
}
