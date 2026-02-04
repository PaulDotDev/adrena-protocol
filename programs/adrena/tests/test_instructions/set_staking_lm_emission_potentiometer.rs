use {
    crate::utils::{self, pda},
    adrena::{instructions::SetStakingLmEmissionPotentiometersParams, state::staking::Staking},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

pub async fn set_staking_lm_emission_potentiometer(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    staked_token_mint: &Pubkey,
    params: SetStakingLmEmissionPotentiometersParams,
) -> std::result::Result<(), BanksClientError> {
    let cortex_pda = pda::get_cortex_pda().0;
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;

    // ==== WHEN ==============================================================
    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::SetStakingLmEmissionPotentiometers {
            admin: admin.pubkey(),
            cortex: cortex_pda,
            staking: staking_pda,
        }
        .to_account_metas(None),
        adrena::instruction::SetStakingLmEmissionPotentiometers {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[admin, payer],
        None,
        None,
    )
    .await?;

    utils::log_instruction(
        "set_staking_lm_emission_potentiometer",
        "admin",
        ix_size,
        tx_used_cu,
    );

    // ==== THEN ==============================================================
    let staking_account = utils::get_account::<Staking>(program_test_ctx, staking_pda).await;

    // Check custody account
    {
        assert_eq!(
            staking_account.lm_emission_potentiometer_bps,
            params.lm_emission_potentiometer_bps
        );
    }

    Ok(())
}
