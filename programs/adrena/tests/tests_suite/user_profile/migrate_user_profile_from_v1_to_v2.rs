use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::InitUserProfileParams,
        state::{
            cortex::Cortex,
            user_profile::{
                legacy::UserProfileV1, Continent, ProfilePicture, Team, Title, UserProfile,
                Wallpaper,
            },
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;

pub async fn migrate_user_profile_from_v1_to_v2() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {},
            },
            utils::UserParam {
                name: "kylian",
                token_balances: hashmap! {},
            },
        ],
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
    let martin = test_setup.get_user_keypair_by_name("martin");
    let paul = test_setup.get_user_keypair_by_name("paul");
    let kylian = test_setup.get_user_keypair_by_name("kylian");

    // Alice: vest 250k token, unlock period from now to in 7 days
    let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

    let (alice_user_profile_pda, user_profile_bump) = pda::get_user_profile_pda(&alice.pubkey());

    // Create martin user_profile account to check already existing username
    test_instructions::init_user_profile(
        &test_setup.program_test_ctx,
        martin.pubkey(),
        martin,
        &test_setup.payer_keypair,
        InitUserProfileParams {
            nickname: "martin".to_string(),
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Team::Default as u8,
            continent: Continent::Default as u8,
        },
        None,
    )
    .await
    .unwrap();

    /************************************************************************************/
    /*********************************** [ ALICE TESTS ] ***********************************/
    /************************************************************************************/

    let alice_user_profile_v1 = UserProfileV1 {
        version: 0, // Should be what's onchain for accounts initialized before we created UserProfileV2 (aka UserProfile)
        bump: user_profile_bump,
        owner: alice.pubkey(),
        _padding: [0; 6],
        nickname: LimitedString::new("alice"),
        created_at: current_time,
        swap_count: 0,
        swap_volume_usd: 0,
        swap_fee_paid_usd: 0,
        short_stats: Default::default(),
        long_stats: Default::default(),
    };

    // Write a fake user_profile account version 1 onchain
    {
        utils::write_adrena_account(
            &test_setup.program_test_ctx,
            &alice_user_profile_v1,
            &alice_user_profile_pda,
            UserProfileV1::LEN,
        )
        .await;
    }

    // Should be able to read the account from onchain as V1
    let alice_user_profile_v1_onchain = utils::get_zero_copy_account::<UserProfileV1>(
        &test_setup.program_test_ctx,
        alice_user_profile_pda,
    )
    .await;

    {
        // Shouldn't mutate theses v1 values
        assert_eq!(
            alice_user_profile_v1_onchain.bump,
            alice_user_profile_v1.bump
        );
        assert_eq!(
            alice_user_profile_v1_onchain.owner,
            alice_user_profile_v1.owner
        );
        assert_eq!(
            alice_user_profile_v1_onchain.nickname,
            alice_user_profile_v1.nickname
        );
        assert_eq!(
            alice_user_profile_v1_onchain.created_at,
            alice_user_profile_v1.created_at
        );
        assert_eq!(
            alice_user_profile_v1_onchain.swap_count,
            alice_user_profile_v1.swap_count
        );
        assert_eq!(
            alice_user_profile_v1_onchain.swap_volume_usd,
            alice_user_profile_v1.swap_volume_usd
        );
        assert_eq!(
            alice_user_profile_v1_onchain.swap_fee_paid_usd,
            alice_user_profile_v1.swap_fee_paid_usd
        );
        assert_eq!(
            alice_user_profile_v1_onchain.short_stats,
            alice_user_profile_v1.short_stats
        );
        assert_eq!(
            alice_user_profile_v1_onchain.long_stats,
            alice_user_profile_v1.long_stats
        );
        assert_eq!(
            alice_user_profile_v1_onchain.version,
            alice_user_profile_v1.version
        );
    }

    // Migrate to v2 should fail when nickname is already used or too long
    {
        assert!(test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,         // martin as caller
            alice.pubkey(), // alice as owner
            "martin",       // Try using martin's nickname
        )
        .await
        .is_err());

        assert!(test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,                        // alice as caller (owner)
            alice.pubkey(),               // alice as owner
            "abcdefghijklmnopqrstuvwxyz", // Too long nickname
        )
        .await
        .is_err());
    }

    // Test successful migration with caller == owner
    {
        println!("Testing migration with caller == owner");
        test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,          // alice as caller (owner)
            alice.pubkey(), // alice as owner
            "alice",        // Keep same nickname
        )
        .await
        .unwrap();

        // Should be able to read the account from onchain as V2
        let alice_user_profile_v2 = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            alice_user_profile_pda,
        )
        .await;

        // Verify migration succeeded
        assert_eq!(alice_user_profile_v2.version, 2);
        assert_eq!(
            alice_user_profile_v2.nickname,
            alice_user_profile_v1.nickname
        );
        assert_eq!(alice_user_profile_v2.title, 0);
        assert_eq!(alice_user_profile_v2.profile_picture, 0);
        assert_eq!(alice_user_profile_v2.wallpaper, 0);
        assert_eq!(alice_user_profile_v2.team, 0);
        assert_eq!(alice_user_profile_v2.continent, 0);
    }

    /************************************************************************************/
    /*********************************** [ PAUL TESTS ] ***********************************/
    /************************************************************************************/

    // Create a second user profile for testing migration by non-owner for same nickname
    let (paul_profile_pda, paul_profile_bump) = pda::get_user_profile_pda(&paul.pubkey());

    let paul_profile_v1: UserProfileV1 = UserProfileV1 {
        version: 0,
        bump: paul_profile_bump,
        owner: paul.pubkey(),
        _padding: [0; 6],
        nickname: LimitedString::new("paul"),
        created_at: current_time,
        swap_count: 0,
        swap_volume_usd: 0,
        swap_fee_paid_usd: 0,
        short_stats: Default::default(),
        long_stats: Default::default(),
    };

    // Write fake user_profile
    {
        utils::write_adrena_account(
            &test_setup.program_test_ctx,
            &paul_profile_v1,
            &paul_profile_pda,
            UserProfileV1::LEN,
        )
        .await;
    }

    // Test failed migration with caller != owner (without permission) because nickname is not in the correct format
    {
        println!("Testing migration with caller != owner (should fail)");
        assert!(test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,        // martin as caller
            paul.pubkey(), // paul as owner
            "AdrenaMaxi",
        )
        .await
        .is_err());
    }

    // should work because keeping same nickname
    {
        println!("Testing migration with caller != owner (should work)");
        test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,        // martin as caller
            paul.pubkey(), // paul as owner
            "paul",
        )
        .await
        .unwrap();

        let paul_user_profile_v2 = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            paul_profile_pda,
        )
        .await;

        assert_eq!(paul_user_profile_v2.version, 2);
        assert_eq!(paul_user_profile_v2.nickname, paul_profile_v1.nickname);
    }

    /************************************************************************************/
    /*********************************** [ KYLIAN TESTS ] *******************************/
    /************************************************************************************/

    // Create a second user profile for testing migration by non-owner for new nickname good format
    let (kylian_profile_pda, kylian_profile_bump) = pda::get_user_profile_pda(&kylian.pubkey());

    let kylian_profile_v1 = UserProfileV1 {
        version: 0,
        bump: kylian_profile_bump,
        owner: kylian.pubkey(),
        _padding: [0; 6],
        nickname: LimitedString::new("Monster123"),
        created_at: current_time,
        swap_count: 0,
        swap_volume_usd: 0,
        swap_fee_paid_usd: 0,
        short_stats: Default::default(),
        long_stats: Default::default(),
    };

    // Write fake user_profile
    {
        utils::write_adrena_account(
            &test_setup.program_test_ctx,
            &kylian_profile_v1,
            &kylian_profile_pda,
            UserProfileV1::LEN,
        )
        .await;
    }

    // Test failed migration with caller != owner (without permission) because nickname is not in the correct format
    {
        println!("Testing migration with caller != owner (should fail)");
        assert!(test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,          // martin as caller
            kylian.pubkey(), // kylian as owner
            "Monster123a",
        )
        .await
        .is_err());
    }

    // should work because new nickname good format
    {
        println!("Testing migration with caller != owner (should work)");
        test_instructions::migrate_user_profile_from_v1_to_v2(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            martin,          // martin as caller
            kylian.pubkey(), // kylian as owner
            "Monster123",
        )
        .await
        .unwrap();

        let kylian_user_profile_v2 = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            kylian_profile_pda,
        )
        .await;

        assert_eq!(kylian_user_profile_v2.version, 2);
        assert_eq!(
            kylian_user_profile_v2.nickname,
            LimitedString::new("Monster123")
        );
    }
}
