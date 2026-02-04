use {
    crate::{
        test_instructions,
        utils::{self, pda, scale, warp_forward, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{AddVestParams, BucketName},
        state::cortex::Cortex,
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
};

const USDC_DECIMALS: u8 = 6;

pub async fn claim() {
    let core_contributor_bucket_starting_allocation = 1_000_000;
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
        utils::scale(
            core_contributor_bucket_starting_allocation,
            Cortex::LM_DECIMALS,
        ),
        utils::scale(3_000_000, Cortex::LM_DECIMALS),
        utils::scale(4_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let alice = test_setup.get_user_keypair_by_name("alice");

    // Alice: vest 250k token, unlock period from now to in 7 days
    let vest_amount = 250_000;
    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;
    let (cortex_pda, _) = pda::get_cortex_pda();

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    let cortex_before =
        utils::get_zero_copy_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

    test_instructions::init_user_staking(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        &lm_token_mint_pda,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    test_instructions::add_vest(
        &test_setup.program_test_ctx,
        &test_setup.admin_keypair,
        &test_setup.payer_keypair,
        alice,
        &AddVestParams {
            amount: utils::scale(vest_amount, Cortex::LM_DECIMALS),
            origin_bucket: BucketName::CoreContributor.into(),
            unlock_start_timestamp: current_time,
            unlock_end_timestamp: utils::days_in_seconds(7) + current_time,
            vote_multiplier: Cortex::BPS_POWER as u32, // x1
        },
    )
    .await
    .unwrap();

    // Check state after vest creation, before claim
    {
        let cortex_after =
            utils::get_zero_copy_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        // Nothing changes yet regarding the amount of token minted
        assert_eq!(
            cortex_after.get_token_amount_left_in_bucket(BucketName::CoreContributor),
            cortex_before.get_token_amount_left_in_bucket(BucketName::CoreContributor)
        );

        // Check that the reserved token are reserved
        assert_eq!(
            cortex_after.get_non_reserved_token_amount_left_in_bucket(BucketName::CoreContributor),
            cortex_after
                .get_token_amount_left_in_bucket(BucketName::CoreContributor)
                .checked_sub(scale(vest_amount, Cortex::LM_DECIMALS))
                .unwrap()
        );
    }

    // move 7 days forward
    warp_forward(&test_setup.program_test_ctx, 7 * 24 * 60 * 60 + 1).await;

    // Alice: claim vest
    test_instructions::claim_vest(
        &test_setup.program_test_ctx,
        &test_setup.payer_keypair,
        alice,
        &test_setup.governance_realm_pda,
    )
    .await
    .unwrap();

    // Verify the internal vest account after claim
    {
        let cortex_after =
            utils::get_zero_copy_account::<Cortex>(&test_setup.program_test_ctx, cortex_pda).await;

        assert_eq!(
            cortex_after.get_token_amount_left_in_bucket(BucketName::CoreContributor),
            cortex_before
                .get_token_amount_left_in_bucket(BucketName::CoreContributor)
                .checked_sub(scale(vest_amount, Cortex::LM_DECIMALS))
                .unwrap()
        );
        // The vest is claimed, so no more reserved tokens
        assert_eq!(
            cortex_after.get_non_reserved_token_amount_left_in_bucket(BucketName::CoreContributor),
            cortex_after.get_token_amount_left_in_bucket(BucketName::CoreContributor)
        );
        // Verify that other buckets didn't move
        {
            assert_eq!(
                cortex_after.get_token_amount_left_in_bucket(BucketName::Foundation),
                cortex_before.get_token_amount_left_in_bucket(BucketName::Foundation),
            );
            assert_eq!(
                cortex_after.get_token_amount_left_in_bucket(BucketName::Ecosystem),
                cortex_before.get_token_amount_left_in_bucket(BucketName::Ecosystem),
            );

            assert_eq!(
                cortex_after.get_non_reserved_token_amount_left_in_bucket(BucketName::Foundation),
                cortex_before.get_non_reserved_token_amount_left_in_bucket(BucketName::Foundation),
            );
            assert_eq!(
                cortex_after.get_non_reserved_token_amount_left_in_bucket(BucketName::Ecosystem),
                cortex_before.get_non_reserved_token_amount_left_in_bucket(BucketName::Ecosystem),
            );
        }
    }
}
