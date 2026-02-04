use {
    crate::utils::{self, pda},
    adrena::{
        instructions::DistributeFeesParams,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::{prelude::AccountMeta, InstructionData, ToAccountMetas},
    solana_program::pubkey::Pubkey,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

async fn get_distribute_fees_accounts_meta(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    pool_pda: &Pubkey,
) -> std::result::Result<Vec<AccountMeta>, BanksClientError> {
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda: Pubkey = pda::get_cortex_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;

    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let lm_staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let lm_staking_reward_token_vault_pda =
        utils::get_staking_reward_token_vault_pda(&lm_staking_pda).0;

    let lp_staking_pda = pda::get_staking_pda(&lp_token_mint_pda).0;
    let lp_staking_reward_token_vault_pda =
        utils::get_staking_reward_token_vault_pda(&lp_staking_pda).0;

    let srt_custody_pda = pda::get_custody_pda(pool_pda, &cortex_account.fee_redistribution_mint).0;
    let srt_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, &cortex_account.fee_redistribution_mint).0;

    let referrer_reward_token_vault_pda =
        pda::get_referrer_reward_token_vault(&cortex_account.fee_redistribution_mint).0;

    // ==== WHEN ==============================================================

    let accounts = adrena::accounts::DistributeFees {
        caller: caller.pubkey(),
        cortex: cortex_pda,
        transfer_authority: transfer_authority_pda,
        pool: *pool_pda,
        lm_staking: lm_staking_pda,
        lp_staking: lp_staking_pda,
        lp_token_mint: lp_token_mint_pda,
        lm_token_mint: lm_token_mint_pda,
        fee_redistribution_mint: cortex_account.fee_redistribution_mint,
        lm_staking_reward_token_vault: lm_staking_reward_token_vault_pda,
        lp_staking_reward_token_vault: lp_staking_reward_token_vault_pda,
        referrer_reward_token_vault: referrer_reward_token_vault_pda,
        staking_reward_token_custody: srt_custody_pda,
        oracle: oracle_pda,
        staking_reward_token_custody_token_account: srt_custody_token_account_pda,
        protocol_fee_recipient: cortex_account.protocol_fee_recipient,
        token_program: anchor_spl::token::ID,
        adrena_program: adrena::ID,
        system_program: solana_program::system_program::id(),
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

    Ok(accounts_meta)
}

pub async fn get_distribute_fees_ix(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    pool_pda: &Pubkey,
    params: &DistributeFeesParams,
) -> std::result::Result<solana_sdk::instruction::Instruction, BanksClientError> {
    let accounts_meta =
        get_distribute_fees_accounts_meta(program_test_ctx, caller, pool_pda).await?;

    Ok(solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: accounts_meta,
        data: adrena::instruction::DistributeFees {
            params: params.clone(),
        }
        .data(),
    })
}

pub async fn distribute_fees(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    params: &DistributeFeesParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== GIVEN =============================================================
    let accounts_meta =
        get_distribute_fees_accounts_meta(program_test_ctx, caller, pool_pda).await?;
    // ==== WHEN ==============================================================

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        accounts_meta,
        adrena::instruction::DistributeFees {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[caller, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("distribute_fees", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    Ok(())
}
