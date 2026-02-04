use {
    super::get_distribute_fees_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::{DistributeFeesParams, ResolvePositionBorrowFeesParams},
        state::{custody::Custody, position::Position, user_profile::UserProfile},
    },
    anchor_lang::{prelude::Pubkey, Id, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn resolve_position_borrow_fees(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    position_pda: &Pubkey,
    params: ResolvePositionBorrowFeesParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;

    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    let pool_pda = &position_account.pool;
    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, position_account.custody).await;

    let custody_token_mint = &custody_account.mint;
    let collateral_custody_pda = position_account.collateral_custody;

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let cortex_pda = pda::get_cortex_pda().0;

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

    utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::ResolvePositionBorrowFees {
            signer: caller.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            collateral_custody: collateral_custody_pda,
            user_profile: if user_profile_account.is_some() {
                Some(user_profile_pda)
            } else {
                None
            },
            referrer_profile: user_profile_referrer_pda,
            token_program: anchor_spl::token::Token::id(),
            adrena_program: adrena::program::Adrena::id(),
        }
        .to_account_metas(None),
        adrena::instruction::ResolvePositionBorrowFees {
            params: params.clone(),
        },
        Some(&caller.pubkey()),
        &[caller],
        None,
        Some(vec![
            get_distribute_fees_ix(
                program_test_ctx,
                caller,
                pool_pda,
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
