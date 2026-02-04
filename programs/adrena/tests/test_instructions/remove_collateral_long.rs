use {
    crate::utils::{self, pda},
    adrena::{
        instructions::RemoveCollateralLongParams,
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

pub async fn remove_collateral_long(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    position_pda: &Pubkey,
    params: RemoveCollateralLongParams,
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
    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, custody_pda).await;
    let custody_token_mint = custody_account.mint;
    let custody_token_account_pda = custody_account.token_account;

    let receiving_account_pda =
        utils::find_associated_token_account(&owner.pubkey(), &custody_token_mint).0;

    // Save account state before tx execution
    let receiving_account_before =
        utils::get_token_account(program_test_ctx, receiving_account_pda).await;
    let custody_token_account_before =
        utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::RemoveCollateralLong {
            owner: owner.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: pool_pda,
            position: *position_pda,
            custody: custody_pda,
            custody_token_account: custody_token_account_pda,
            oracle: oracle_pda,
            receiving_account: receiving_account_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::RemoveCollateralLong {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("remove_collateral_long", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    // Check the balance change
    {
        let receiving_account_after =
            utils::get_token_account(program_test_ctx, receiving_account_pda).await;
        let custody_token_account_after =
            utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

        assert!(receiving_account_after.amount > receiving_account_before.amount);
        assert!(custody_token_account_after.amount < custody_token_account_before.amount);
    }

    // Check the position to ensure collaterall has been removed
    {
        let position_account_after =
            utils::get_account::<Position>(program_test_ctx, *position_pda).await;

        assert_eq!(position_account_after.owner, owner.pubkey());
        assert_eq!(position_account_after.pool, pool_pda);
        assert_eq!(position_account_after.custody, custody_pda);
        assert!(position_account_after.update_time != 0);
        assert_eq!(position_account_after.get_side(), Side::Long);
        assert_eq!(
            position_account_after.collateral_usd,
            position_account.collateral_usd - params.collateral_usd
        );
    }

    Ok(())
}
