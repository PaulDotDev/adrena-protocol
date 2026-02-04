use {
    crate::utils::{self, pda},
    adrena::state::{chaos_labs_oracle::ChaosLabsBatchPrices, pool::Pool},
    anchor_lang::{prelude::Pubkey, InstructionData, ToAccountMetas},
    solana_program::instruction::AccountMeta,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn get_update_pool_ix(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<solana_sdk::instruction::Instruction, BanksClientError> {
    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;

    let accounts_meta = {
        let accounts = adrena::accounts::UpdatePoolAum {
            payer: payer.pubkey(),
            cortex: cortex_pda,
            pool: *pool_pda,
            oracle: oracle_pda,
            lp_token_mint: lp_token_mint_pda,
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

    let ix = solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: accounts_meta,
        data: adrena::instruction::UpdatePoolAum {
            params: adrena::instructions::UpdatePoolAumParams {
                oracle_prices: oracle_prices.clone(),
            },
        }
        .data(),
    };

    Ok(ix)
}
