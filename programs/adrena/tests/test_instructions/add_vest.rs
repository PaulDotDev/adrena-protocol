use {
    super::get_sync_user_voting_power_ix,
    crate::utils::{self, pda},
    adrena::{
        instructions::AddVestParams,
        state::{vest::Vest, vest_registry::VestRegistry},
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn add_vest(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    owner: &Keypair,
    params: &AddVestParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let vest_registry_pda = pda::get_vest_registry_pda().0;
    let (vest_pda, vest_bump) = pda::get_vest_pda(&owner.pubkey());
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let vest_registry_account_before =
        utils::get_account::<VestRegistry>(program_test_ctx, vest_registry_pda).await;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::AddVest {
            admin: admin.pubkey(),
            owner: owner.pubkey(),
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            vest_registry: vest_registry_pda,
            vest: vest_pda,
            lm_token_mint: lm_token_mint_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            rent: solana_program::sysvar::rent::ID,
        }
        .to_account_metas(None),
        adrena::instruction::AddVest { params: *params },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        Some(vec![
            get_sync_user_voting_power_ix(program_test_ctx, admin, &owner.pubkey(), payer, true)
                .await?,
        ]),
    )
    .await?;

    utils::log_instruction("add_vest", "admin", ix_size, tx_used_cu);

    // ==== THEN ==============================================================

    // Check vest account
    {
        let vest_account = utils::get_zero_copy_account::<Vest>(program_test_ctx, vest_pda).await;

        assert_eq!(vest_account.amount, params.amount);
        assert_eq!(
            vest_account.unlock_start_timestamp,
            params.unlock_start_timestamp
        );
        assert_eq!(
            vest_account.unlock_end_timestamp,
            params.unlock_end_timestamp
        );
        assert_eq!(vest_account.claimed_amount, 0);
        assert_eq!(vest_account.last_claim_timestamp, 0);
        assert_eq!(vest_account.owner, owner.pubkey());
        assert_eq!(vest_account.bump, vest_bump);
    }

    // Check vest_registry account
    {
        let vest_registry_account_after =
            utils::get_account::<VestRegistry>(program_test_ctx, vest_registry_pda).await;

        assert_eq!(*vest_registry_account_after.vests.last().unwrap(), vest_pda);

        assert_eq!(
            vest_registry_account_before.vesting_token_amount + params.amount,
            vest_registry_account_after.vesting_token_amount
        );
        assert_eq!(
            vest_registry_account_before.vested_token_amount,
            vest_registry_account_after.vested_token_amount
        );
    }

    Ok((vest_pda, vest_bump))
}
