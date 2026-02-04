use {
    crate::utils::{self, pda},
    adrena::{
        instructions::AddCollateralLongParams,
        state::position::{Position, Side},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn add_collateral_long(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    params: AddCollateralLongParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint).0;
    let cortex_pda = pda::get_cortex_pda().0;
    let (position_pda, position_bump) =
        pda::get_position_pda(&owner.pubkey(), pool_pda, &custody_pda, Side::Long);

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), custody_token_mint).0;

    // Save account state before tx execution
    let owner_funding_account_before =
        utils::get_token_account(program_test_ctx, funding_account_address).await;
    let custody_token_account_before =
        utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

    // To check the position change later
    let position_account_before =
        utils::get_account::<Position>(program_test_ctx, position_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddCollateralLong {
            owner: owner.pubkey(),
            funding_account: funding_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            position: position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            custody_token_account: custody_token_account_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddCollateralLong {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_collateral_long", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    // Check the balance change
    {
        let owner_funding_account_after =
            utils::get_token_account(program_test_ctx, funding_account_address).await;
        let custody_token_account_after =
            utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

        assert!(owner_funding_account_after.amount < owner_funding_account_before.amount);
        assert!(custody_token_account_after.amount > custody_token_account_before.amount);
    }

    // Check the position to ensure collaterall has been added
    {
        let position_account_after =
            utils::get_account::<Position>(program_test_ctx, position_pda).await;

        assert_eq!(position_account_after.owner, owner.pubkey());
        assert_eq!(position_account_after.pool, *pool_pda);
        assert_eq!(position_account_after.custody, custody_pda);
        assert!(position_account_after.update_time != 0);
        assert_eq!(position_account_after.get_side(), Side::Long);
        assert_eq!(
            position_account_after.collateral_amount,
            position_account_before.collateral_amount + params.collateral
        );
    }

    Ok((position_pda, position_bump))
}
