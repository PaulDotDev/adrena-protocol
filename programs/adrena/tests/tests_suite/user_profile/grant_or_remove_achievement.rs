use {
    crate::{
        test_instructions::{edit_user_profile, init_user_profile},
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            public::user_profile_p::grant_or_remove_achievement::GrantOrRemoveAchievementParams,
            EditUserProfileParams, InitUserProfileParams,
        },
        state::{
            cortex::Cortex,
            user_profile::{
                Achievement, Continent, ProfilePicture, Team, Title, UserProfile, Wallpaper,
            },
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::signer::Signer,
    std::sync::Arc,
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

// Use our test_instructions grant_achievement
use crate::test_instructions::grant_or_remove_achievement;

struct TestContext {
    test_setup: Arc<utils::TestSetup>,
    user: solana_sdk::signature::Keypair,
    payer: solana_sdk::signature::Keypair,
}

pub async fn grant_or_remove_achievement_tests() {
    // Create test setup once and reuse it for all tests
    let test_context = setup_test_env().await;

    // Run all tests with the shared test context
    test_grant_single_achievement_to_empty_profile(&test_context).await;
    test_grant_achievement_to_profile_with_existing_achievements(&test_context).await;
    test_grant_already_unlocked_achievement(&test_context).await;
    test_grant_invalid_achievement(&test_context).await;
    test_remove_single_achievement(&test_context).await;
    test_remove_multiple_achievements(&test_context).await;
    test_remove_nonexistent_achievement(&test_context).await;
    test_toggle_achievement_status(&test_context).await;
    test_edit_profile_after_setting_achievements(&test_context).await;
    test_empty_achievement_vector(&test_context).await;
    test_duplicate_achievements(&test_context).await;
    test_achievement_points(&test_context).await;
}

async fn setup_test_env() -> TestContext {
    // Setup test environment with minimal configuration
    let test_setup = Arc::new(
        utils::TestSetup::new(
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
        .await,
    );

    let user = test_setup
        .get_user_keypair_by_name("alice")
        .insecure_clone();

    let payer = test_setup.payer_keypair.insecure_clone();

    // First create a user profile (just once for all tests)
    init_user_profile(
        &test_setup.program_test_ctx,
        user.pubkey(),
        &user,
        &payer,
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

    // Verify the user profile was properly initialized
    let user_profile_pda = utils::pda::get_user_profile_pda(&user.pubkey()).0;
    let mut ctx_lock = test_setup.program_test_ctx.write().await;

    match ctx_lock.banks_client.get_account(user_profile_pda).await {
        Ok(Some(account)) => {
            assert_eq!(
                account.owner,
                adrena::id(),
                "User profile has incorrect owner"
            );
        }
        Ok(None) => {
            panic!("User profile account doesn't exist after initialization!");
        }
        Err(e) => {
            panic!("Error fetching user profile after initialization: {:?}", e);
        }
    }

    drop(ctx_lock);

    TestContext {
        test_setup,
        user,
        payer,
    }
}

async fn test_grant_single_achievement_to_empty_profile(ctx: &TestContext) {
    // Grant the achievement using our test_instructions function
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::FirstTrade as u8],
            operation: 1,
        },
    )
    .await
    .unwrap();

    // Verify success
    let user_profile_pda = utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0;
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        user_profile_pda,
    )
    .await;

    assert!(
        user_profile.has_achievement(Achievement::FirstTrade),
        "Achievement should be added to user profile"
    );
}

async fn test_grant_achievement_to_profile_with_existing_achievements(ctx: &TestContext) {
    println!("Running test_grant_achievement_to_profile_with_existing_achievements");

    // Grant the initial achievements
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![
                Achievement::FirstProfitableTrade as u8,
                Achievement::Volume1M as u8,
                Achievement::Loss5K as u8,
                Achievement::Profit5K as u8,
            ],
            operation: 1,
        },
    )
    .await
    .unwrap();

    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::Volume10M as u8],
            operation: 1,
        },
    )
    .await
    .unwrap();

    // Verify the achievements were added
    let user_profile_pda = utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0;
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        user_profile_pda,
    )
    .await;

    assert!(
        user_profile.has_achievement(Achievement::FirstProfitableTrade),
        "FirstProfitableTrade should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::Volume1M),
        "Volume1M should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::Loss5K),
        "Loss5K should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::Profit5K),
        "Profit5K should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::Volume10M),
        "Volume10M should be added"
    );
}

async fn test_grant_already_unlocked_achievement(ctx: &TestContext) {
    // First, add an achievement if it doesn't exist
    if !utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await
    .has_achievement(Achievement::ChangeUsername10)
    {
        grant_or_remove_achievement(
            &ctx.test_setup.program_test_ctx,
            &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
            &ctx.payer,
            &ctx.user.pubkey(),
            GrantOrRemoveAchievementParams {
                achievements: vec![Achievement::ChangeUsername10 as u8],
                operation: 1,
            },
        )
        .await
        .unwrap();
    }

    // Try to add the same achievement again
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::ChangeUsername10 as u8],
            operation: 1,
        },
    )
    .await
    .unwrap();
}

async fn test_grant_invalid_achievement(ctx: &TestContext) {
    // Try to add an invalid achievement (outside the valid range)
    assert!(grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![255],
            operation: 1,
        },
    )
    .await
    .is_err());
}

async fn test_remove_single_achievement(ctx: &TestContext) {
    // First, grant the Streak5 achievement if not already there
    if !utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await
    .has_achievement(Achievement::Streak5)
    {
        grant_or_remove_achievement(
            &ctx.test_setup.program_test_ctx,
            &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
            &ctx.payer,
            &ctx.user.pubkey(),
            GrantOrRemoveAchievementParams {
                achievements: vec![Achievement::Streak5 as u8],
                operation: 1,
            },
        )
        .await
        .unwrap();
    }

    // Now remove the achievement
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::Streak5 as u8],
            operation: 2,
        },
    )
    .await
    .unwrap();

    // Verify achievement was removed
    let user_profile_pda = utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0;
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        user_profile_pda,
    )
    .await;

    assert!(
        !user_profile.has_achievement(Achievement::Streak5),
        "Streak5 achievement should be removed"
    );
}

async fn test_remove_multiple_achievements(ctx: &TestContext) {
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![
                Achievement::Streak10 as u8,
                Achievement::StakedEarnings10 as u8,
                Achievement::Liquidated1 as u8,
            ],
            operation: 1,
        },
    )
    .await
    .unwrap();

    // Verify achievements were added
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert!(
        user_profile.has_achievement(Achievement::Streak10),
        "Streak10 should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::StakedEarnings10),
        "StakedEarnings10 should be added"
    );
    assert!(
        user_profile.has_achievement(Achievement::Liquidated1),
        "Liquidated1 should be added"
    );

    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::Streak10 as u8, Achievement::Liquidated1 as u8],
            operation: 2,
        },
    )
    .await
    .unwrap();

    // Verify correct achievements were removed
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert!(
        !user_profile.has_achievement(Achievement::Streak10),
        "Streak10 should be removed"
    );
    assert!(
        user_profile.has_achievement(Achievement::StakedEarnings10),
        "StakedEarnings10 should still exist"
    );
    assert!(
        !user_profile.has_achievement(Achievement::Liquidated1),
        "Liquidated1 should be removed"
    );
}

async fn test_remove_nonexistent_achievement(ctx: &TestContext) {
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::StakedHoldings50M as u8],
            operation: 2,
        },
    )
    .await
    .unwrap();

    // Try to remove an achievement that doesn't exist in the profile
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::StakedHoldings50M as u8],
            operation: 2,
        },
    )
    .await
    .unwrap();
}

async fn test_toggle_achievement_status(ctx: &TestContext) {
    // Test achievement
    let achievement_id = Achievement::TradeOpen30Days as u8;

    // 1. GRANT: First grant the achievement
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![achievement_id],
            operation: 1,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // 2. REMOVE: Then remove it
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![achievement_id],
            operation: 2,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // 3. RE-GRANT: Try to add it back with a new params object
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![achievement_id],
            operation: 1,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // Verify achievement was re-added
    let user_profile = utils::get_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert!(
        user_profile.has_achievement(Achievement::TradeOpen30Days),
        "Achievement should be re-added"
    );
}

async fn test_edit_profile_after_setting_achievements(ctx: &TestContext) {
    // Grant the FirstTrade achievement to unlock Trader title
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair, // For whitelisted_caller parameter (will be ignored)
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::FirstTrade as u8],
            operation: 1,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // Edit the profile's wallpaper and title
    edit_user_profile(
        &ctx.test_setup.program_test_ctx,
        &ctx.user,
        &ctx.payer,
        EditUserProfileParams {
            profile_picture: ProfilePicture::One as u8,
            wallpaper: Wallpaper::One as u8,
            title: Title::Trader as u8,
            team: None,
            continent: None,
        },
        None,
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // Verify the profile's state matches the edits
    let user_profile_pda = utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0;
    let user_profile = utils::get_zero_copy_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        user_profile_pda,
    )
    .await;

    assert_eq!(
        user_profile.profile_picture,
        ProfilePicture::One as u8,
        "Profile picture should be updated"
    );
    assert_eq!(
        user_profile.wallpaper,
        Wallpaper::One as u8,
        "Wallpaper should be updated"
    );
    assert_eq!(
        user_profile.title,
        Title::Trader as u8,
        "Title should be updated"
    );
}

async fn test_empty_achievement_vector(ctx: &TestContext) {
    // Try to grant with empty vector
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![],
            operation: 1,
        },
    )
    .await
    .unwrap();
}

async fn test_duplicate_achievements(ctx: &TestContext) {
    // Try to grant same achievement twice in one request
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::FirstTrade as u8, Achievement::FirstTrade as u8],
            operation: 1,
        },
    )
    .await
    .unwrap();

    // Verify only one instance was granted
    let user_profile = utils::get_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert!(
        user_profile.has_achievement(Achievement::FirstTrade),
        "Achievement should be granted"
    );
}

async fn test_achievement_points(ctx: &TestContext) {
    println!("Running test_achievement_points");

    // Grant a high-point achievement
    let params = GrantOrRemoveAchievementParams {
        achievements: vec![Achievement::Volume1B as u8], // 200 points
        operation: 1,
    };

    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        params,
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    // Verify points are set correctly
    let user_profile = utils::get_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert_eq!(
        user_profile.achievements[Achievement::Volume1B as usize],
        Achievement::Volume1B.points() as u8,
        "Achievement points should be set correctly"
    );

    // Remove and verify points are cleared
    grant_or_remove_achievement(
        &ctx.test_setup.program_test_ctx,
        &ctx.test_setup.payer_keypair,
        &ctx.payer,
        &ctx.user.pubkey(),
        GrantOrRemoveAchievementParams {
            achievements: vec![Achievement::Volume1B as u8],
            operation: 2,
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&ctx.test_setup.program_test_ctx, 1).await;

    let user_profile = utils::get_account::<UserProfile>(
        &ctx.test_setup.program_test_ctx,
        utils::pda::get_user_profile_pda(&ctx.user.pubkey()).0,
    )
    .await;

    assert_eq!(
        user_profile.achievements[Achievement::Volume1B as usize],
        0,
        "Achievement points should be cleared on removal"
    );
}
