use {
    crate::utils::{self, pda},
    adrena::{
        instructions::AddPoolPartOneParams,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    std::str::FromStr,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn add_pool_part_one(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_name: &str,
    aum_soft_cap_usd: u64,
) -> std::result::Result<(Pubkey, u8, Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let (pool_pda, pool_bump) = pda::get_pool_pda(&String::from_str(pool_name).unwrap());
    let (lp_token_mint_pda, lp_token_mint_bump) = pda::get_lp_token_mint_pda(&pool_pda);
    let (lp_token_mint_metadata_pda, _) = pda::get_metadata_pda(&lp_token_mint_pda);

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddPoolPartOne {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: pool_pda,
            lp_token_mint: lp_token_mint_pda,
            lp_token_mint_metadata: lp_token_mint_metadata_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            mpl_token_metadata_program: mpl_token_metadata::ID,
            rent: solana_program::sysvar::rent::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddPoolPartOne {
            params: AddPoolPartOneParams {
                name: String::from_str(pool_name).unwrap(),
                aum_soft_cap_usd,
                lp_token_name: "Adrena LP".to_string(),
                lp_token_symbol: "ALP".to_string(),
                lp_token_uri: "https://arweave.net/fRrkJvBTj9ZMa3j1sy5HMEvBNVIP0MDfonXXyKfbSW4"
                    .to_string(),
            },
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_pool_part_one", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, pool_pda).await;

    assert_eq!(pool_account.name.to_string().as_str(), pool_name);
    assert_eq!(pool_account.bump, pool_bump);
    assert_eq!(pool_account.lp_token_bump, lp_token_mint_bump);
    assert_eq!(pool_account.initialized, false as u8);

    let cortex_account = utils::get_zero_copy_account::<Cortex>(program_test_ctx, cortex_pda).await;

    assert!(cortex_account.pools.iter().any(|p| pool_pda.eq(p)));

    Ok((pool_pda, pool_bump, lp_token_mint_pda, lp_token_mint_bump))
}
