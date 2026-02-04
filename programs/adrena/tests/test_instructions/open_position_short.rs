use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::OpenPositionShortParams,
        state::position::{Position, Side},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn open_position_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    collateral_custody_token_mint: &Pubkey,
    params: OpenPositionShortParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;

    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;

    let collateral_custody_pda = pda::get_custody_pda(pool_pda, collateral_custody_token_mint).0;

    let collateral_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, collateral_custody_token_mint).0;

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), collateral_custody_token_mint).0;

    let (position_pda, position_bump) =
        pda::get_position_pda(&owner.pubkey(), pool_pda, &custody_pda, Side::Short);

    let cortex_pda = pda::get_cortex_pda().0;

    // Save account state before tx execution
    let owner_funding_account_before =
        utils::get_token_account(program_test_ctx, funding_account_address).await;

    let collateral_custody_token_account_before =
        utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::OpenPositionShort {
            owner: owner.pubkey(),
            caller: owner.pubkey(),
            payer: payer.pubkey(),
            funding_account: funding_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            position: position_pda,
            custody: custody_pda,
            collateral_custody: collateral_custody_pda,
            oracle: oracle_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::OpenPositionShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        Some(vec![
            get_update_pool_ix(program_test_ctx, payer, pool_pda, params.oracle_prices).await?,
        ]),
        None,
    )
    .await?;

    utils::log_instruction("open_position_short", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    // Check the balance change
    {
        let owner_funding_account_after =
            utils::get_token_account(program_test_ctx, funding_account_address).await;
        let collateral_custody_token_account_after =
            utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

        assert!(owner_funding_account_after.amount < owner_funding_account_before.amount);
        assert!(
            collateral_custody_token_account_after.amount
                > collateral_custody_token_account_before.amount
        );
    }

    // Check the position
    {
        let position_account =
            utils::get_zero_copy_account::<Position>(program_test_ctx, position_pda).await;

        assert_eq!(position_account.owner, owner.pubkey());
        assert_eq!(position_account.pool, *pool_pda);
        assert_eq!(position_account.custody, custody_pda);
        // Need to handle test/not test case
        // assert_eq!(
        //     position_account.open_time,
        //     utils::get_current_unix_timestamp(program_test_ctx).await
        // );
        assert_eq!(position_account.update_time, 0);
        assert_eq!(position_account.get_side(), Side::Short);
        assert_eq!(position_account.unrealized_interest_usd, 0);
        assert!(position_account.collateral_amount <= params.collateral); // to account for fees -  <= to accommodate tests without fees
        assert_eq!(position_account.bump, position_bump);
    }

    Ok((position_pda, position_bump))
}
