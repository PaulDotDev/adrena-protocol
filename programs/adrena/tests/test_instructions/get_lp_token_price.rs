use {
    super::get_update_pool_ix,
    crate::utils::{self, pda},
    adrena::state::{chaos_labs_oracle::ChaosLabsBatchPrices, pool::Pool},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program::instruction::AccountMeta,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn get_lp_token_price(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    lp_token_mint_pda: &Pubkey,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<u64, BanksClientError> {
    // ==== WHEN ==============================================================
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    let accounts_meta = {
        let accounts = adrena::accounts::GetLpTokenPrice {
            cortex: cortex_pda,
            pool: *pool_pda,
            lp_token_mint: *lp_token_mint_pda,
            oracle: oracle_pda,
        };

        let mut accounts_meta = accounts.to_account_metas(None);

        let pool_account = utils::get_zero_copy_account::<Pool>(program_test_ctx, *pool_pda).await;

        // For each token, add custody account as remaining_account
        for custody in &pool_account.custodies {
            if custody.ne(&Pubkey::default()) {
                accounts_meta.push(AccountMeta {
                    pubkey: *custody,
                    is_signer: false,
                    is_writable: false,
                });
            }
        }

        accounts_meta
    };

    let result: u64 = utils::create_and_simulate_cortex_view_ix(
        program_test_ctx,
        accounts_meta,
        adrena::instruction::GetLpTokenPrice {
            params: adrena::instructions::GetLpTokenPriceParams {
                oracle_prices: oracle_prices.clone(),
            },
        },
        payer,
        Some(get_update_pool_ix(program_test_ctx, payer, pool_pda, oracle_prices).await?),
        None,
    )
    .await?;

    // ==== THEN ==============================================================
    Ok(result)
}
