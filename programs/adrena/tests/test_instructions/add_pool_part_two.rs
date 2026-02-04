use {
    crate::utils::{self, pda},
    adrena::{
        instructions::{AddPoolPartTwoParams, ReservedSpots},
        state::{cortex::Cortex, genesis_lock::GenesisLock, pool::Pool},
    },
    anchor_lang::ToAccountMetas,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    std::str::FromStr,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn add_pool_part_two(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_name: &str,
    genesis_lock_campaign_duration: i64,
    genesis_lock_campaign_start_date: i64,
    genesis_reserved_grant_duration: i64,
    _first_reserved_spot: &Pubkey,
    _second_reserved_spot: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let (pool_pda, pool_bump) = pda::get_pool_pda(&String::from_str(pool_name).unwrap());
    let (lp_token_mint_pda, lp_token_mint_bump) = pda::get_lp_token_mint_pda(&pool_pda);
    let (genesis_lock_pda, genesis_lock_bump) = pda::get_genesis_lock_pda(&pool_pda);

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddPoolPartTwo {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: pool_pda,
            lp_token_mint: lp_token_mint_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            genesis_lock: genesis_lock_pda,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddPoolPartTwo {
            params: AddPoolPartTwoParams {
                genesis_lock_campaign_duration,
                genesis_reserved_grant_duration,
                genesis_lock_campaign_start_date,
                #[cfg(feature = "test")]
                reserved_spots: ReservedSpots::Test {
                    first_reserved_spot: *_first_reserved_spot,
                    second_reserved_spot: *_second_reserved_spot,
                },
                #[cfg(not(feature = "test"))]
                reserved_spots: ReservedSpots::None,
            },
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_pool_part_two", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, pool_pda).await;

    assert_eq!(pool_account.name.to_string().as_str(), pool_name);
    assert_eq!(pool_account.bump, pool_bump);
    assert_eq!(pool_account.lp_token_bump, lp_token_mint_bump);
    assert_eq!(pool_account.initialized, true as u8);

    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    assert!(cortex_account.pools.iter().any(|p| pool_pda.eq(p)));

    let genesis_lock_account =
        utils::get_zero_copy_account::<GenesisLock>(program_test_ctx, genesis_lock_pda).await;

    // Assert genesis lock
    {
        assert_eq!(genesis_lock_account.bump, genesis_lock_bump);
        assert_eq!(
            genesis_lock_account.campaign_duration,
            genesis_lock_campaign_duration
        );
        assert_eq!(
            genesis_lock_account.campaign_start_date,
            genesis_lock_campaign_start_date
        );
        assert_eq!(
            genesis_lock_account.public_amount,
            GenesisLock::CAMPAIGN_PUBLIC_USDC_AMOUNT
        );
        assert_eq!(
            genesis_lock_account.reserved_amount,
            GenesisLock::CAMPAIGN_RESERVED_USDC_AMOUNT
        );
        assert_eq!(genesis_lock_account.public_amount_claimed, 0);
        assert_eq!(genesis_lock_account.reserved_amount_claimed, 0);
    }

    Ok(())
}
