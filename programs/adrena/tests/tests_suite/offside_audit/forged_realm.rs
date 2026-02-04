use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        adapters::spl_governance_program_adapter,
        instructions::{AddLiquidStakeParams, AddVestParams, BucketName},
        state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
    },
    spl_governance::state::{
        enums::MintMaxVoterWeightSource,
        realm::{GoverningTokenConfigAccountArgs, RealmV2},
        realm_config::GoverningTokenType,
    },
    tokio::sync::RwLock,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn forged_realm() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(3_000, USDC_DECIMALS),
                    "eth" => utils::scale(2, ETH_DECIMALS),
                },
            },
        ],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
            },
            utils::MintParam {
                name: "eth",
                decimals: ETH_DECIMALS,
            },
        ],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(10_000_000, Cortex::USD_DECIMALS),
        vec![
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "usdc",
                    is_stable: true,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1, Cortex::PRICE_DECIMALS),
                        initial_conf: 10000000, // 10 bps
                        oracle_name: LimitedString::new("usdc"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::USDC,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(1_500, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "eth",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(1500, Cortex::PRICE_DECIMALS),
                        initial_conf: 15000000000, // 10 bps
                        oracle_name: LimitedString::new("eth"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::ETH,
                    },
                    trade_oracle: None,
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(1, ETH_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    {
        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();
    }

    // Prep work: Alice get 2 governance tokens using vesting
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            alice,
            &AddVestParams {
                amount: utils::scale(2, Cortex::LM_DECIMALS),
                origin_bucket: BucketName::CoreContributor.into(),
                unlock_start_timestamp: current_time,
                unlock_end_timestamp: current_time + utils::days_in_seconds(7),
                vote_multiplier: Cortex::BPS_POWER as u32, // x1
            },
        )
        .await
        .unwrap();

        // Move until vest end
        utils::warp_forward(&test_setup.program_test_ctx, utils::days_in_seconds(7) + 1).await;

        test_instructions::claim_vest(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    let gov_token_mint_pda = pda::get_governance_token_mint_pda().0;

    // lets start,
    // forge a new realm
    let forged_realm_pda = forge_realm(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        "whatever".to_string(),
        1,
        &gov_token_mint_pda,
    )
    .await
    .unwrap();

    // Alice: add LM liquid staking
    {
        assert!(test_instructions::add_liquid_stake(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            AddLiquidStakeParams {
                amount: utils::scale(1, Cortex::LM_DECIMALS),
            },
            &forged_realm_pda,
            &test_setup.pool_pda,
            &lm_token_mint_pda,
        )
        .await
        .is_err());
    }

    // let gov_token_ata = utils::initialize_token_accounts(
    //     &test_setup.program_test_ctx,
    //     gov_token_mint_pda,
    //     &test_setup.payer_keypair,
    //     &[alice.pubkey()],
    // ).await.unwrap()[0];

    // withdraw_gov_token_from_forge_realm(
    //     &test_setup.program_test_ctx,
    //     alice,
    //     &test_setup.payer_keypair,
    //     &forged_realm_pda,
    //     &gov_token_ata,
    //     &gov_token_mint_pda,
    // ).await.unwrap();

    // utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // // Remove the other half of the stake
    // test_instructions::remove_liquid_stake(
    //     &test_setup.program_test_ctx,
    //     alice,
    //     &test_setup.payer_keypair,
    //     RemoveLiquidStakeParams {
    //         amount: utils::scale(1, Cortex::LM_DECIMALS),
    //     },
    //     &cortex_stake_reward_mint,
    //     &test_setup.governance_realm_pda,
    //     &test_setup.pool_pda,
    //     &lm_token_mint_pda,
    // )
    // .await
    // .unwrap();

    // let lm_balance = utils::get_token_account_balance(
    //     &test_setup.program_test_ctx,
    //     alice_lm_token_account_address,
    // )
    // .await;

    // let gov_token_balance = utils::get_token_account_balance(
    //     &test_setup.program_test_ctx,
    //     gov_token_ata,
    // )
    // .await;

    // println!("{} {}", lm_balance, gov_token_balance);
}

pub async fn forge_realm(
    program_test_ctx: &RwLock<ProgramTestContext>,
    admin: &Keypair,
    payer: &Keypair,
    name: String,
    min_community_weight_to_create_governance: u64,
    community_token_mint: &Pubkey,
) -> std::result::Result<Pubkey, BanksClientError> {
    let realm_pda = pda::get_governance_realm_pda(name.clone());

    let mut ctx: tokio::sync::RwLockWriteGuard<'_, ProgramTestContext> =
        program_test_ctx.write().await;
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[spl_governance::instruction::create_realm(
            &spl_governance_program_adapter::id(),
            &admin.pubkey(),
            community_token_mint,
            &payer.pubkey(),
            None,
            Some(GoverningTokenConfigAccountArgs {
                token_type: GoverningTokenType::Liquid,
                voter_weight_addin: None,
                max_voter_weight_addin: None,
            }),
            None,
            name,
            min_community_weight_to_create_governance,
            MintMaxVoterWeightSource::SupplyFraction(100),
        )],
        Some(&payer.pubkey()),
        &[payer],
        last_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    drop(ctx);

    {
        let realm = utils::get_borsh_account::<RealmV2>(program_test_ctx, &realm_pda).await;

        assert_eq!(realm.community_mint, *community_token_mint);
        assert_eq!(realm.authority.unwrap(), admin.pubkey());
    }

    Ok(realm_pda)
}

pub async fn withdraw_gov_token_from_forge_realm(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    realm_pda: &Pubkey,
    dest: &Pubkey,
    community_token_mint: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    let mut ctx: tokio::sync::RwLockWriteGuard<'_, ProgramTestContext> =
        program_test_ctx.write().await;
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[spl_governance::instruction::withdraw_governing_tokens(
            &spl_governance_program_adapter::id(),
            realm_pda,
            dest,
            &owner.pubkey(),
            community_token_mint,
        )],
        Some(&payer.pubkey()),
        &[payer, owner],
        last_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    Ok(())
}
