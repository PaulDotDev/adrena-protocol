use {
    crate::utils::{self, pda},
    adrena::{
        adapters::spl_governance_program_adapter,
        state::{cortex::Cortex, vest::Vest},
    },
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    tokio::sync::RwLock,
};

pub async fn get_sync_user_voting_power_ix(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    owner: &Pubkey,
    payer: &Keypair,
    // Use this when this instruction is bundled with add_vest
    force_vest: bool,
) -> std::result::Result<solana_sdk::instruction::Instruction, BanksClientError> {
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let vest_registry_pda = pda::get_vest_registry_pda().0;
    let vest_pda = pda::get_vest_pda(owner).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;
    let staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;

    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let vest_account = utils::try_get_account::<Vest>(program_test_ctx, vest_pda).await;

    let governance_governing_token_holding_pda = pda::get_governance_governing_token_holding_pda(
        &cortex_account.governance_realm,
        &governance_token_mint_pda,
    );

    let governance_realm_config_pda =
        pda::get_governance_realm_config_pda(&cortex_account.governance_realm);

    let governance_governing_token_owner_record_pda =
        pda::get_governance_governing_token_owner_record_pda(
            &cortex_account.governance_realm,
            &governance_token_mint_pda,
            owner,
        );

    let ix = solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: adrena::accounts::SyncUserVotingPower {
            caller: caller.pubkey(),
            owner: *owner,
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            vest_registry: vest_registry_pda,
            vest: if vest_account.is_some() || force_vest {
                Some(vest_pda)
            } else {
                None
            },
            lm_token_mint: lm_token_mint_pda,
            governance_token_mint: governance_token_mint_pda,
            governance_realm: cortex_account.governance_realm,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            governance_program: spl_governance_program_adapter::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        data: adrena::instruction::SyncUserVotingPower {}.data(),
    };

    Ok(ix)
}

pub async fn sync_user_voting_power(
    program_test_ctx: &RwLock<ProgramTestContext>,
    caller: &Keypair,
    owner: &Pubkey,
    payer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;
    let vest_registry_pda = pda::get_vest_registry_pda().0;
    let vest_pda = pda::get_vest_pda(owner).0;
    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;
    let governance_token_mint_pda = pda::get_governance_token_mint_pda().0;
    let staking_pda = pda::get_staking_pda(&lm_token_mint_pda).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;

    let cortex_account = utils::get_account::<Cortex>(program_test_ctx, cortex_pda).await;

    let vest_account = utils::try_get_account::<Vest>(program_test_ctx, vest_pda).await;

    let governance_governing_token_holding_pda = pda::get_governance_governing_token_holding_pda(
        &cortex_account.governance_realm,
        &governance_token_mint_pda,
    );

    let governance_realm_config_pda =
        pda::get_governance_realm_config_pda(&cortex_account.governance_realm);

    let governance_governing_token_owner_record_pda =
        pda::get_governance_governing_token_owner_record_pda(
            &cortex_account.governance_realm,
            &governance_token_mint_pda,
            owner,
        );

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SyncUserVotingPower {
            caller: caller.pubkey(),
            owner: *owner,
            payer: payer.pubkey(),
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            vest_registry: vest_registry_pda,
            vest: if vest_account.is_some() {
                Some(vest_pda)
            } else {
                None
            },
            lm_token_mint: lm_token_mint_pda,
            governance_token_mint: governance_token_mint_pda,
            governance_realm: cortex_account.governance_realm,
            governance_realm_config: governance_realm_config_pda,
            governance_governing_token_holding: governance_governing_token_holding_pda,
            governance_governing_token_owner_record: governance_governing_token_owner_record_pda,
            user_staking: user_staking_pda,
            staking: staking_pda,
            governance_program: spl_governance_program_adapter::ID,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::SyncUserVotingPower {},
        Some(&payer.pubkey()),
        &[caller, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction("sync_user_voting_power", "public", ix_size, tx_used_cu);

    // ==== THEN ==============================================================
    Ok(())
}
