use {
    crate::{
        test_instructions::get_distribute_fees_ix,
        utils::{self, pda},
    },
    adrena::{
        instructions::{AddLiquidityParams, DistributeFeesParams},
        state::pool::Pool,
    },
    anchor_lang::{
        prelude::{AccountMeta, Pubkey},
        ToAccountMetas,
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn add_liquidity(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    params: AddLiquidityParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint).0;
    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lp_staking_pda = pda::get_staking_pda(&lp_token_mint_pda).0;

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), custody_token_mint).0;
    let lp_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lp_token_mint_pda).0;

    let lm_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lm_staking_pda).0;

    let lp_staking_reward_token_vault_pda =
        pda::get_staking_reward_token_vault_pda(&lp_staking_pda).0;

    // Save account state before tx execution
    let owner_funding_account_before =
        utils::get_token_account(program_test_ctx, funding_account_address).await;
    let owner_lp_token_account_before =
        utils::get_token_account(program_test_ctx, lp_token_account_address).await;
    let custody_token_account_before =
        utils::get_token_account(program_test_ctx, custody_token_account_pda).await;
    let lm_staking_reward_token_vault_account_before =
        utils::get_token_account(program_test_ctx, lm_staking_reward_token_vault_pda).await;
    let lp_staking_reward_token_vault_account_before =
        utils::get_token_account(program_test_ctx, lp_staking_reward_token_vault_pda).await;

    let accounts_meta = {
        let accounts = adrena::accounts::AddLiquidity {
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            lp_token_account: lp_token_account_address,
            transfer_authority: transfer_authority_pda,
            lp_staking: lp_staking_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            custody_token_account: custody_token_account_pda,
            lp_token_mint: lp_token_mint_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        };

        let mut accounts_meta = accounts.to_account_metas(None);

        let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, *pool_pda).await;

        // For each token, add custody account as remaining_account
        for custody in &pool_account.custodies {
            if custody.ne(&Pubkey::default()) {
                accounts_meta.push(AccountMeta {
                    pubkey: *custody,
                    is_signer: false,
                    is_writable: false,
                });
            }
        }

        accounts_meta
    };

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        accounts_meta,
        adrena::instruction::AddLiquidity {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        Some(vec![
            get_distribute_fees_ix(
                program_test_ctx,
                payer,
                pool_pda,
                &DistributeFeesParams {
                    oracle_prices: params.oracle_prices,
                },
            )
            .await?,
        ]),
    )
    .await?;

    utils::log_instruction("add_liquidity", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let owner_funding_account_after =
        utils::get_token_account(program_test_ctx, funding_account_address).await;
    let owner_lp_token_account_after =
        utils::get_token_account(program_test_ctx, lp_token_account_address).await;
    let custody_token_account_after =
        utils::get_token_account(program_test_ctx, custody_token_account_pda).await;
    let lm_staking_reward_token_vault_account_after =
        utils::get_token_account(program_test_ctx, lm_staking_reward_token_vault_pda).await;
    let lp_staking_reward_token_vault_account_after =
        utils::get_token_account(program_test_ctx, lp_staking_reward_token_vault_pda).await;

    assert!(owner_funding_account_after.amount < owner_funding_account_before.amount);
    assert!(owner_lp_token_account_after.amount > owner_lp_token_account_before.amount);
    assert!(custody_token_account_after.amount > custody_token_account_before.amount);
    assert!(
        lm_staking_reward_token_vault_account_after.amount
            >= lm_staking_reward_token_vault_account_before.amount
    );
    // Only increase if there are locked staked lp tokens
    assert!(
        lp_staking_reward_token_vault_account_after.amount
            >= lp_staking_reward_token_vault_account_before.amount
    );

    Ok(())
}
