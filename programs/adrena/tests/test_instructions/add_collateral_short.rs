use {
    crate::utils::{self, pda},
    adrena::{
        instructions::AddCollateralShortParams,
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

pub async fn add_collateral_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    position_pda: &Pubkey,
    params: AddCollateralShortParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    let pool_pda = position_account.pool;
    let cortex_pda = pda::get_cortex_pda().0;
    let custody_pda = position_account.custody;

    // custody collateral retrieval
    let collateral_custody_pda = position_account.collateral_custody;
    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;
    let collateral_custody_token_mint = collateral_custody_account.mint;
    let collateral_custody_token_account_pda = pda::get_custody_token_account_pda(
        &position_account.pool,
        &collateral_custody_account.mint,
    )
    .0;

    let funding_account_pda =
        utils::find_associated_token_account(&owner.pubkey(), &collateral_custody_token_mint).0;

    // Save account state before tx execution
    let owner_funding_account_before =
        utils::get_token_account(program_test_ctx, funding_account_pda).await;
    let collateral_custody_token_account_pda_before =
        utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

    // To check the position change later
    let position_account_before =
        utils::get_account::<Position>(program_test_ctx, *position_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddCollateralShort {
            owner: owner.pubkey(),
            funding_account: funding_account_pda,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: pool_pda,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_custody: collateral_custody_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddCollateralShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_collateral_short", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    // Check the balance change
    {
        let owner_funding_account_after =
            utils::get_token_account(program_test_ctx, funding_account_pda).await;
        let collateral_custody_token_account_after =
            utils::get_token_account(program_test_ctx, collateral_custody_token_account_pda).await;

        assert!(owner_funding_account_after.amount < owner_funding_account_before.amount);
        assert!(
            collateral_custody_token_account_after.amount
                > collateral_custody_token_account_pda_before.amount
        );
    }

    // Check the position to ensure collaterall has been added
    {
        let position_account_after =
            utils::get_account::<Position>(program_test_ctx, *position_pda).await;

        assert_eq!(position_account_after.owner, owner.pubkey());
        assert_eq!(position_account_after.pool, pool_pda);
        assert_eq!(position_account_after.custody, custody_pda);
        assert!(position_account_after.update_time != 0);
        assert_eq!(position_account_after.get_side(), Side::Short);
        assert_eq!(
            position_account_after.collateral_amount,
            position_account_before.collateral_amount + params.collateral
        );
    }

    Ok(())
}
