use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter,
        instructions::AddLiquidStakeParams,
        state::{cortex::Cortex, staking::Staking, user_staking::UserStaking},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn add_liquid_stake(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    params: AddLiquidStakeParams,
    governance_realm_pda: &Pubkey,
    pool_pda: &Pubkey,
    staked_token_mint: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(&owner.pubkey(), &staking_pda).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let genesis_lock_pda = pda::get_genesis_lock_pda(pool_pda).0;
    let staking_staked_token_vault_pda = pda::get_staking_staked_token_vault_pda(&staking_pda).0;
    let staking_reward_token_vault_pda = pda::get_staking_reward_token_vault_pda(&staking_pda).0;
    let staking_lm_reward_token_vault_pda =
        pda::get_staking_lm_reward_token_vault_pda(&staking_pda).0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), staked_token_mint).0;

    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let reward_token_account_address = utils::find_associated_token_account(
        &owner.pubkey(),
        &cortex_account.fee_redistribution_mint,
    )
    .0;
    let lm_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lm_token_mint_pda).0;

    let governance_governing_token_holding_pda = pda::get_governance_governing_token_holding_pda(
        governance_realm_pda,
        &governance_token_mint_pda,
    );

    let governance_realm_config_pda = pda::get_governance_realm_config_pda(governance_realm_pda);

    let governance_governing_token_owner_record_pda =
        pda::get_governance_governing_token_owner_record_pda(
            governance_realm_pda,
            &governance_token_mint_pda,
            &owner.pubkey(),
        );

    // // ==== WHEN ==============================================================
    // save account state before tx execution
    let staking_account_before = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;

    let user_staking_account_before =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let governance_governing_token_holding_balance_before =
        utils::get_token_account_balance(program_test_ctx, governance_governing_token_holding_pda)
            .await;

    let funding_account_before =
        utils::get_token_account_balance(program_test_ctx, funding_account_address).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddLiquidStake {
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            reward_token_account: reward_token_account_address,
            lm_token_account: lm_token_account_address,
            staking_staked_token_vault: staking_staked_token_vault_pda,
            staking_reward_token_vault: staking_reward_token_vault_pda,
            staking_lm_reward_token_vault: staking_lm_reward_token_vault_pda,
            transfer_authority: transfer_authority_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            genesis_lock: genesis_lock_pda,
            lm_token_mint: lm_token_mint_pda,
            governance_token_mint: governance_token_mint_pda,
            fee_redistribution_mint: cortex_account.fee_redistribution_mint,
            governance_realm: *governance_realm_pda,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            governance_program: spl_governance_program_adapter::ID,
            adrena_program: adrena::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddLiquidStake {
            params: AddLiquidStakeParams {
                amount: params.amount,
            },
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_liquid_stake", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let governance_governing_token_holding_balance_after =
        utils::get_token_account_balance(program_test_ctx, governance_governing_token_holding_pda)
            .await;

    let staking_account_after = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;

    let user_staking_account_after =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let funding_account_after =
        utils::get_token_account_balance(program_test_ctx, funding_account_address).await;

    // Check changes in staking account
    {
        assert!(
            user_staking_account_after.liquid_stake.amount
                > user_staking_account_before.liquid_stake.amount,
        );

        assert_eq!(
            staking_account_after.nb_liquid_tokens - staking_account_before.nb_liquid_tokens,
            params.amount
        );
    }

    // Check staked token ATA balance
    {
        assert_eq!(
            funding_account_before - params.amount,
            funding_account_after,
        );
    }

    // Check voting power
    {
        assert_eq!(
            governance_governing_token_holding_balance_before + params.amount,
            governance_governing_token_holding_balance_after,
        );
    }

    Ok(())
}
