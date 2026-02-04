use {
    crate::{
        test_instructions::{get_distribute_fees_ix, get_update_pool_ix},
        utils::{self, pda},
    },
    adrena::instructions::{DistributeFeesParams, SwapParams},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn swap(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    // Mint received by the User
    dispensing_custody_token_mint: &Pubkey,
    // Mint sent by the User
    receiving_custody_token_mint: &Pubkey,
    params: SwapParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let dispensing_custody_pda = pda::get_custody_pda(pool_pda, dispensing_custody_token_mint).0;
    let dispensing_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, dispensing_custody_token_mint).0;
    let receiving_custody_pda = pda::get_custody_pda(pool_pda, receiving_custody_token_mint).0;
    let receiving_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, receiving_custody_token_mint).0;

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), receiving_custody_token_mint).0;
    let receiving_account_address =
        utils::find_associated_token_account(&owner.pubkey(), dispensing_custody_token_mint).0;

    // Save account state before tx execution
    let owner_funding_account_before =
        utils::get_token_account(program_test_ctx, funding_account_address).await;
    let custody_receiving_account_before =
        utils::get_token_account(program_test_ctx, receiving_account_address).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::Swap {
            caller: owner.pubkey(),
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            receiving_custody: receiving_custody_pda,
            receiving_custody_token_account: receiving_custody_token_account_pda,
            dispensing_custody: dispensing_custody_pda,
            oracle: oracle_pda,
            dispensing_custody_token_account: dispensing_custody_token_account_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::Swap {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        Some(vec![
            get_update_pool_ix(
                program_test_ctx,
                payer,
                pool_pda,
                params.oracle_prices.clone(),
            )
            .await?,
        ]),
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

    utils::log_instruction("swap", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    // Check the balance change
    let owner_funding_account_after =
        utils::get_token_account(program_test_ctx, funding_account_address).await;
    let custody_receiving_account_after =
        utils::get_token_account(program_test_ctx, receiving_account_address).await;

    assert!(owner_funding_account_after.amount < owner_funding_account_before.amount);
    assert!(custody_receiving_account_after.amount > custody_receiving_account_before.amount);

    Ok(())
}
