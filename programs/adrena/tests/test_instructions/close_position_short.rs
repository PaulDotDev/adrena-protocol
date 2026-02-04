use {
    super::get_distribute_fees_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::{ClosePositionShortParams, DistributeFeesParams},
        state::{custody::Custody, position::Position, user_profile::UserProfile},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn close_position_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    position_pda: &Pubkey,
    params: ClosePositionShortParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;
    let custody_pda = position_account.custody;
    let collateral_custody_pda = position_account.collateral_custody;

    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;

    let collateral_custody_token_account_pda = pda::get_custody_token_account_pda(
        &position_account.pool,
        &collateral_custody_account.mint,
    )
    .0;

    let cortex_pda = pda::get_cortex_pda().0;

    let receiving_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &collateral_custody_account.mint).0;

    let user_profile_pda = pda::get_user_profile_pda(&position_account.owner).0;

    let user_profile_account =
        utils::try_get_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    let user_profile_referrer_pda = if let Some(user_profile_account) = user_profile_account {
        if user_profile_account.referrer_profile.ne(&Pubkey::default()) {
            Some(user_profile_account.referrer_profile)
        } else {
            None
        }
    } else {
        None
    };

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ClosePositionShort {
            caller: owner.pubkey(),
            owner: owner.pubkey(),
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: position_account.pool,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_custody: collateral_custody_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            user_profile: if user_profile_account.is_some() {
                Some(user_profile_pda)
            } else {
                None
            },
            referrer_profile: user_profile_referrer_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::ClosePositionShort { params },
        Some(&payer.pubkey()),
        &[owner, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("close_position_short", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    Ok(())
}

pub async fn close_position_short_permissionless(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    owner: &Pubkey,
    position_pda: &Pubkey,
    params: ClosePositionShortParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;
    let custody_pda = position_account.custody;
    let collateral_custody_pda = position_account.collateral_custody;

    let collateral_custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, collateral_custody_pda).await;

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;

    let collateral_custody_token_account_pda = pda::get_custody_token_account_pda(
        &position_account.pool,
        &collateral_custody_account.mint,
    )
    .0;

    let cortex_pda = pda::get_cortex_pda().0;

    let receiving_account_address =
        utils::find_associated_token_account(owner, &collateral_custody_account.mint).0;

    let user_profile_pda = pda::get_user_profile_pda(&position_account.owner).0;

    let user_profile_account =
        utils::try_get_account::<UserProfile>(program_test_ctx, user_profile_pda).await;

    let user_profile_referrer_pda = if let Some(user_profile_account) = user_profile_account {
        if user_profile_account.referrer_profile.ne(&Pubkey::default()) {
            Some(user_profile_account.referrer_profile)
        } else {
            None
        }
    } else {
        None
    };

    let position = utils::get_account::<Position>(program_test_ctx, *position_pda).await;

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ClosePositionShort {
            caller: payer.pubkey(),
            owner: *owner,
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: position_account.pool,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_custody: collateral_custody_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            user_profile: if user_profile_account.is_some() {
                Some(user_profile_pda)
            } else {
                None
            },
            referrer_profile: user_profile_referrer_pda,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::ClosePositionShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[payer],
        None,
        Some(vec![
            get_distribute_fees_ix(
                program_test_ctx,
                payer,
                &position.pool,
                &DistributeFeesParams {
                    oracle_prices: params.oracle_prices,
                },
            )
            .await?,
        ]),
    )
    .await?;

    // ==== THEN ==============================================================

    Ok(())
}
