use {
    super::{get_distribute_fees_ix, get_update_pool_ix},
    crate::utils::{self, pda},
    adrena::{
        instructions::LiquidateLongParams,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, custody::Custody, position::Position,
            user_profile::UserProfile,
        },
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn liquidate_long(
    program_test_ctx: &RwLock<ProgramTestContext>,
    liquidator: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    position_pda: &Pubkey,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;

    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, custody_pda).await;

    let owner = position_account.owner;

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let custody_token_account_pda =
        pda::get_custody_token_account_pda(&position_account.pool, &custody_account.mint).0;

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

    let receiving_account_address =
        utils::find_associated_token_account(&owner, custody_token_mint).0;

    // Save account state before tx execution
    let receiving_account_before =
        utils::get_token_account(program_test_ctx, receiving_account_address).await;
    let custody_token_account_before =
        utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

    let position = utils::get_account::<Position>(program_test_ctx, *position_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::LiquidateLong {
            signer: liquidator.pubkey(),
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: position_account.pool,
            position: *position_pda,
            custody: custody_pda,
            oracle: oracle_pda,
            custody_token_account: custody_token_account_pda,
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
        adrena::instruction::LiquidateLong {
            params: LiquidateLongParams {
                oracle_prices: oracle_prices.clone(),
            },
        },
        Some(&payer.pubkey()),
        &[liquidator, payer],
        Some(vec![
            get_update_pool_ix(program_test_ctx, payer, pool_pda, oracle_prices.clone()).await?,
        ]),
        Some(vec![
            get_distribute_fees_ix(
                program_test_ctx,
                payer,
                &position.pool,
                &adrena::instructions::DistributeFeesParams { oracle_prices },
            )
            .await?,
        ]),
    )
    .await?;

    utils::log_instruction("liquidate_long", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    // Check the balance change
    {
        let receiving_account_after =
            utils::get_token_account(program_test_ctx, receiving_account_address).await;
        let custody_token_account_after =
            utils::get_token_account(program_test_ctx, custody_token_account_pda).await;

        assert!(receiving_account_after.amount >= receiving_account_before.amount);
        assert!(custody_token_account_after.amount <= custody_token_account_before.amount);
    }

    Ok(())
}
