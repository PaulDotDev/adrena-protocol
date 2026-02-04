use {
    crate::utils::{self, pda},
    adrena::{
        instructions::AddCustodyParams,
        state::{custody::Custody, pool::Pool},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn add_custody(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    custody_token_decimals: u8,
    params: AddCustodyParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let (custody_pda, custody_bump) = pda::get_custody_pda(pool_pda, custody_token_mint);
    let (custody_token_account_pda, custody_token_account_bump) =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint);

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddCustody {
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            oracle: oracle_pda,
            pool: *pool_pda,
            custody: custody_pda,
            custody_token_account: custody_token_account_pda,
            custody_token_mint: *custody_token_mint,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddCustody {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("add_custody", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    let custody_account =
        utils::get_zero_copy_account::<Custody>(program_test_ctx, custody_pda).await;

    // Check custody account
    {
        assert_eq!(custody_account.pool, *pool_pda);
        assert_eq!(custody_account.mint, *custody_token_mint);
        assert_eq!(custody_account.token_account, custody_token_account_pda);
        assert_eq!(custody_account.decimals, custody_token_decimals);
        assert_eq!(custody_account.is_stable, params.is_stable as u8);
        assert_eq!(custody_account.pricing, params.pricing);
        assert_eq!(custody_account.allow_swap, params.allow_swap as u8);
        assert_eq!(custody_account.allow_trade, params.allow_trade as u8);
        assert_eq!(custody_account.fees, params.fees);
        assert_eq!(custody_account.borrow_rate, params.borrow_rate,);
        assert_eq!(custody_account.bump, custody_bump);
        assert_eq!(
            custody_account.token_account_bump,
            custody_token_account_bump
        );
    }

    let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, *pool_pda).await;

    // Check pool token
    {
        let idx = pool_account.get_token_id(&custody_pda).unwrap();
        let custody = pool_account.custodies[idx];
        let ratios = pool_account.ratios[idx];

        assert_eq!(custody, custody_pda);
        assert_eq!(ratios.target, params.ratios[idx].target);
        assert_eq!(ratios.min, params.ratios[idx].min);
        assert_eq!(ratios.max, params.ratios[idx].max);
    }

    Ok((custody_pda, custody_bump))
}
