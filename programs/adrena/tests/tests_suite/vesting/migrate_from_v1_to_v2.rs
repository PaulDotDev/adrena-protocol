use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::BucketName,
        state::{
            cortex::Cortex,
            vest::{legacy::VestV1, Vest},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::{pubkey::Pubkey, signer::Signer},
};

const USDC_DECIMALS: u8 = 6;

pub async fn migrate_from_v1_to_v2() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {},
        }],
        vec![utils::MintParam {
            name: "usdc",
            decimals: USDC_DECIMALS,
        }],
        "usdc",
        6,
        "ADRENA",
        "main_pool",
        utils::scale(10_000_000, Cortex::USD_DECIMALS),
        vec![utils::SetupCustodyWithLiquidityParams {
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
            liquidity_amount: utils::scale(0, USDC_DECIMALS),
            payer_user_name: "alice",
        }],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(3_000_000, Cortex::LM_DECIMALS),
        utils::scale(4_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    // Alice: vest 250k token, unlock period from now to in 7 days
    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

    let (vest_pda, vest_bump) = pda::get_vest_pda(&alice.pubkey());

    let vest_v1 = VestV1 {
        version: 0, // Should be what's onchain for accounts initialized before we created VestV2 (aka Vest)
        bump: vest_bump,
        origin_bucket: BucketName::CoreContributor.into(),
        cancelled: false as u8,
        vote_multiplier: 5000,
        amount: 100000000,
        unlock_start_timestamp: current_time,
        unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
        claimed_amount: 0,
        last_claim_timestamp: current_time,
        owner: alice.pubkey(),
    };

    // Write a fake vest account version 1 onchain
    {
        utils::write_adrena_account(
            &test_setup.program_test_ctx,
            &vest_v1,
            &vest_pda,
            VestV1::LEN,
        )
        .await;
    }

    // Should be able to read the account from onchain as V1
    let vest_v1_onchain =
        utils::get_zero_copy_account::<VestV1>(&test_setup.program_test_ctx, vest_pda).await;

    {
        // Shouldn't mutate theses v1 values
        assert_eq!(vest_v1_onchain.bump, vest_v1.bump);
        assert_eq!(vest_v1_onchain.amount, vest_v1.amount);
        assert_eq!(vest_v1_onchain.claimed_amount, vest_v1.claimed_amount);
        assert_eq!(vest_v1_onchain.cancelled, vest_v1.cancelled);
        assert_eq!(
            vest_v1_onchain.last_claim_timestamp,
            vest_v1.last_claim_timestamp
        );
        assert_eq!(vest_v1_onchain.vote_multiplier, vest_v1.vote_multiplier);
        assert_eq!(vest_v1_onchain.version, vest_v1.version);
    }

    // Migrate to v2
    {
        test_instructions::migrate_vest_from_v1_to_v2(
            &test_setup.program_test_ctx,
            alice,
            &test_setup.payer_keypair,
            alice,
        )
        .await
        .unwrap();
    }

    // Should be able to read the account from onchain as V2
    let vest_v2 =
        utils::get_zero_copy_account::<Vest>(&test_setup.program_test_ctx, vest_pda).await;

    {
        // Shouldn't mutate theses v1 values
        assert_eq!(vest_v2.bump, vest_v1.bump);
        assert_eq!(vest_v2.amount, vest_v1.amount);
        assert_eq!(vest_v2.claimed_amount, vest_v1.claimed_amount);
        assert_eq!(vest_v2.cancelled, vest_v1.cancelled);
        assert_eq!(vest_v2.last_claim_timestamp, vest_v1.last_claim_timestamp);
        assert_eq!(vest_v2.vote_multiplier, vest_v1.vote_multiplier);

        // Should mutate theses v1 values
        assert_eq!(vest_v2.version, 2);

        // Should add theses new values
        assert_eq!(vest_v2.delegate, Pubkey::default());
        assert_eq!(vest_v2.has_delegate, 0);
    }
}
