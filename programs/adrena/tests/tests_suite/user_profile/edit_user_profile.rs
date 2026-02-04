use {
    crate::{
        test_instructions,
        utils::{self, pda, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddVestParams, BucketName, EditUserProfileNicknameParams, EditUserProfileParams,
            InitUserProfileParams,
        },
        state::{
            cortex::Cortex,
            user_profile::{Continent, ProfilePicture, Team, Title, UserProfile, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn edit_user_profile() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(100_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(100_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
                },
            },
            utils::UserParam {
                name: "paul",
                token_balances: hashmap! {
                    "usdc" => utils::scale(100_000, USDC_DECIMALS),
                    "eth" => utils::scale(200, ETH_DECIMALS),
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
                liquidity_amount: utils::scale(15_000, USDC_DECIMALS),
                payer_user_name: "paul",
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
                liquidity_amount: utils::scale(10, ETH_DECIMALS),
                payer_user_name: "paul",
            },
        ],
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        utils::scale(1_000_000, Cortex::LM_DECIMALS),
        None,
        None,
    )
    .await;

    let martin = test_setup.get_user_keypair_by_name("martin");
    let alice = test_setup.get_user_keypair_by_name("alice");

    let lm_token_mint_pda = pda::get_lm_token_mint_pda().0;

    test_instructions::init_user_profile(
        &test_setup.program_test_ctx,
        alice.pubkey(),
        alice,
        &test_setup.payer_keypair,
        InitUserProfileParams {
            nickname: "alice".to_string(),
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

    let martin_profile_pda = test_instructions::init_user_profile(
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
    .unwrap()
    .0;

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

        test_instructions::init_user_staking(
            &test_setup.program_test_ctx,
            martin,
            &test_setup.payer_keypair,
            &lm_token_mint_pda,
            &test_setup.pool_pda,
        )
        .await
        .unwrap();
    }

    // Prep work: Martin get governance tokens using vesting
    {
        let current_time = utils::get_current_unix_timestamp(&test_setup.program_test_ctx).await;

        test_instructions::add_vest(
            &test_setup.program_test_ctx,
            &test_setup.admin_keypair,
            &test_setup.payer_keypair,
            martin,
            &AddVestParams {
                amount: UserProfile::CHANGE_NICKNAME_TAX,
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
            martin,
            &test_setup.governance_realm_pda,
        )
        .await
        .unwrap();
    }

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check initial profile
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.nickname, LimitedString::new("martin"));
    }

    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: None,
            continent: None,
        },
        None,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check updated profile
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.profile_picture, ProfilePicture::Zero as u8);
        assert_eq!(after.wallpaper, Wallpaper::Zero as u8);
    }

    // Test setting team for first time
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Some(Team::Bonk as u8),
            continent: None,
        },
        None,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check team was updated
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.team, Team::Bonk as u8);
    }

    // Try changing team after it's set (should fail)
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Some(Team::Jito as u8),
            continent: None,
        },
        None,
    )
    .await
    .unwrap_err();

    // Test setting continent to a valid value
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: None,
            continent: Some(Continent::Europe as u8),
        },
        None,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.continent, Continent::Europe as u8);
    }

    // Create another user profile for testing both team and continent at once
    let paul = test_setup.get_user_keypair_by_name("paul");

    let paul_profile_pda = test_instructions::init_user_profile(
        &test_setup.program_test_ctx,
        paul.pubkey(),
        paul,
        &test_setup.payer_keypair,
        InitUserProfileParams {
            nickname: "paul".to_string(),
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Team::Default as u8,
            continent: Continent::Default as u8,
        },
        None,
    )
    .await
    .unwrap()
    .0;

    // Test setting both team and continent at the same time
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        paul,
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Some(Team::Jito as u8),
            continent: Some(Continent::NorthAmerica as u8),
        },
        None,
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check both values were updated
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            paul_profile_pda,
        )
        .await;

        assert_eq!(after.team, Team::Jito as u8);
        assert_eq!(after.continent, Continent::NorthAmerica as u8);
    }

    // Test that setting team to default (0) should fail
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        test_setup.get_user_keypair_by_name("alice"),
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Some(Team::Default as u8),
            continent: None,
        },
        None,
    )
    .await
    .unwrap_err();

    // Test that setting continent to default (0) should fail
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        test_setup.get_user_keypair_by_name("alice"),
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: None,
            continent: Some(Continent::Default as u8),
        },
        None,
    )
    .await
    .unwrap_err();

    // Test that setting both team and continent to default (0) should fail
    test_instructions::edit_user_profile(
        &test_setup.program_test_ctx,
        test_setup.get_user_keypair_by_name("alice"),
        &test_setup.payer_keypair,
        EditUserProfileParams {
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Some(Team::Default as u8),
            continent: Some(Continent::Default as u8),
        },
        None,
    )
    .await
    .unwrap_err();

    // Set the same nickname again should fail
    assert!(test_instructions::edit_user_profile_nickname(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileNicknameParams {
            nickname: "martin".to_string(),
        },
    )
    .await
    .is_err());

    // Set an existing nickname should fail
    assert!(test_instructions::edit_user_profile_nickname(
        &test_setup.program_test_ctx,
        alice,
        &test_setup.payer_keypair,
        EditUserProfileNicknameParams {
            nickname: "martin".to_string(),
        },
    )
    .await
    .is_err());

    // should pay the burn ADX to change to this name
    test_instructions::edit_user_profile_nickname(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileNicknameParams {
            nickname: "Monster123456".to_string(),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Check updated profile
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.profile_picture, ProfilePicture::Zero as u8);
        assert_eq!(after.wallpaper, Wallpaper::Zero as u8);
        assert_eq!(after.nickname.to_string(), "Monster123456");
    }

    // Check that changing again doesn't fail despite no amount of adx because format doesn't require
    test_instructions::edit_user_profile_nickname(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileNicknameParams {
            nickname: "martin3".to_string(),
        },
    )
    .await
    .unwrap();

    // Check updated profile
    {
        let after = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            martin_profile_pda,
        )
        .await;

        assert_eq!(after.nickname.to_string(), "martin3");
    }

    // Check that changing again fails because no amount of adx
    test_instructions::edit_user_profile_nickname(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        EditUserProfileNicknameParams {
            nickname: "martin4".to_string(),
        },
    )
    .await
    .unwrap_err();
}
