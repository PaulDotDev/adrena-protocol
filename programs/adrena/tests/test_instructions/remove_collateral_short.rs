use {
    crate::utils::{self, pda},
    adrena::{
        instructions::RemoveCollateralShortParams,
        state::{
            custody::Custody,
            position::{Position, Side},
        },
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn remove_collateral_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    position_pda: &Pubkey,
    params: RemoveCollateralShortParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    let pool_pda = position_account.pool;
    let cortex_pda = pda::get_cortex_pda().0;

    // custody retrieval
    let custody_pda = position_account.custody;

    // custody collateral retrieval
    let collateral_custody_pda = position_account.collateral_custody;
    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;
    let collateral_custody_token_mint = &collateral_custody_account.mint;
    let collateral_custody_token_account_pda =
        pda::get_custody_token_account_pda(&position_account.pool, collateral_custody_token_mint).0;

    let receiving_account_pda =
        utils::find_associated_token_account(&owner.pubkey(), collateral_custody_token_mint).0;

    // Save account state before tx execution
    let receiving_account_before =
        utils::get_token_account(program_test_ctx, receiving_account_pda).await;
    let collateral_custody_token_account_before =
        utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

    // To check the position change later
    let position_account_before =
        utils::get_account::<Position>(program_test_ctx, *position_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::RemoveCollateralShort {
            owner: owner.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: pool_pda,
            position: *position_pda,
            custody: custody_pda,
            collateral_custody: collateral_custody_pda,
            oracle: oracle_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            receiving_account: receiving_account_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::RemoveCollateralShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("remove_collateral_short", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    // Check the balance change
    {
        let receiving_account_after =
            utils::get_token_account(program_test_ctx, receiving_account_pda).await;
        let collateral_custody_token_account_after =
            utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

        assert!(receiving_account_after.amount > receiving_account_before.amount);
        assert!(
            collateral_custody_token_account_after.amount
                < collateral_custody_token_account_before.amount
        );
    }

    // Check the position to ensure collaterall has been removed
    {
        let position_account_after =
            utils::get_account::<Position>(program_test_ctx, *position_pda).await;

        assert_eq!(position_account_after.owner, owner.pubkey());
        assert_eq!(position_account_after.pool, pool_pda);
        assert_eq!(position_account_after.custody, custody_pda);
        assert!(position_account_after.update_time != 0);
        assert_eq!(position_account_after.get_side(), Side::Short);
        assert_eq!(
            position_account_after.collateral_usd,
            position_account_before.collateral_usd - params.collateral_usd
        );
    }

    Ok(())
}
