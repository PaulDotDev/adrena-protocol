use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{BucketName, MintLmTokensFromBucketParams},
        state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn mint_lm_tokens_from_bucket() {
    let test_setup = utils::TestSetup::new(
        vec![utils::UserParam {
            name: "alice",
            token_balances: hashmap! {
                "usdc" => utils::scale(100_000, USDC_DECIMALS),
                "eth" => utils::scale(50, ETH_DECIMALS),
            },
        }],
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
                liquidity_amount: utils::scale(0, USDC_DECIMALS),
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
                liquidity_amount: utils::scale(0, ETH_DECIMALS),
                payer_user_name: "alice",
            },
        ],
        utils::scale(360000000, Cortex::LM_DECIMALS),
        utils::scale(90000000, Cortex::LM_DECIMALS),
        utils::scale(550000000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    test_instructions::mint_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintLmTokensFromBucketParams {
            bucket_name: BucketName::CoreContributor.into(),
            amount: utils::scale(180000000, Cortex::LM_DECIMALS),
            reason: "Mint 50% of core contributor bucket allocation".to_string(),
        },
    )
    .await
    .unwrap();

    assert!(test_instructions::mint_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintLmTokensFromBucketParams {
            bucket_name: BucketName::CoreContributor.into(),
            amount: utils::scale(216000000, Cortex::LM_DECIMALS),
            reason: "Mint 60% of core contributor bucket allocation should fail as we already minted 50%".to_string(),
        },
    )
    .await
    .is_err());

    test_instructions::mint_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintLmTokensFromBucketParams {
            bucket_name: BucketName::CoreContributor.into(),
            amount: utils::scale(180000000, Cortex::LM_DECIMALS),
            reason: "Mint other 50% of core contributor bucket allocation".to_string(),
        },
    )
    .await
    .unwrap();

    test_instructions::mint_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintLmTokensFromBucketParams {
            bucket_name: BucketName::Foundation.into(),
            amount: utils::scale(90000000, Cortex::LM_DECIMALS),
            reason: "Mint 100% of foundation bucket allocation".to_string(),
        },
    )
    .await
    .unwrap();

    assert!(test_instructions::mint_lm_tokens_from_bucket(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &alice.pubkey(),
        &test_setup.payer_keypair,
        MintLmTokensFromBucketParams {
            bucket_name: BucketName::Ecosystem.into(),
            amount: utils::scale(0, Cortex::LM_DECIMALS),
            reason: "Mint 0 should fail".to_string(),
        },
    )
    .await
    .is_err());
}
