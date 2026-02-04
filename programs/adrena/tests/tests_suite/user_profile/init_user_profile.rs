use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::InitUserProfileParams,
        state::{
            cortex::Cortex,
            user_profile::{Continent, ProfilePicture, Team, Title, UserProfile, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    maplit::hashmap,
    solana_sdk::{pubkey::Pubkey, signer::Signer},
};

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn init_user_profile() {
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
            utils::UserParam {
                name: "bob",
                token_balances: hashmap! {
                    "usdc" => utils::scale(1, USDC_DECIMALS),
                    "eth" => utils::scale(1, ETH_DECIMALS),
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
                        initial_conf: 10000000,
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

    let alice = test_setup.get_user_keypair_by_name("alice");
    let bob = test_setup.get_user_keypair_by_name("bob");

    // Test 1: Regular (non-permissionless) profile creation
    {
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
    }

    // Test 2: Permissionless profile creation with valid Monster pattern
    {
        // Create bob's profile permissionlessly from alice as payer
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            bob.pubkey(),
            bob,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Monster123".to_string(),
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
    }

    // Test 3: Permissionless profile creation with invalid nickname format (should fail)
    {
        let random_pubkey = Pubkey::new_unique();
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            random_pubkey,
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Blabla".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            None,
        )
        .await
        .unwrap_err();
    }

    // Test 4: Permissionless profile creation with invalid nickname format (should fail) => No numbers after "Monster"
    {
        let random_pubkey = Pubkey::new_unique();
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            random_pubkey,
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Monster".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            None,
        )
        .await
        .unwrap_err();
    }

    // Test 5: Permissionless profile creation with invalid nickname format (should fail) => Has characters after numbers
    {
        let random_pubkey = Pubkey::new_unique();
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            random_pubkey,
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Monster123abc".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            None,
        )
        .await
        .unwrap_err();
    }

    // Test 6: Permissionless profile creation with non-default values (should fail)
    {
        let random_pubkey = Pubkey::new_unique();
        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            random_pubkey,
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Monster123".to_string(),
                profile_picture: ProfilePicture::One as u8,
                wallpaper: Wallpaper::One as u8,
                title: Title::GoldenHands as u8,
                team: Team::Bonk as u8,
                continent: Continent::Europe as u8,
            },
            None,
        )
        .await
        .unwrap_err();
    }

    // Test 7: Permissionless profile creation with referrer (should fail)
    {
        let random_pubkey = Pubkey::new_unique();
        let alice_profile = utils::pda::get_user_profile_pda(&alice.pubkey()).0;

        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            random_pubkey,
            alice,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "Monster123".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            Some(alice_profile),
        )
        .await
        .unwrap_err();
    }

    // Test 8: Regular profile creation with referrer (should work)
    {
        // First create a user for use as referrer
        let charlie = solana_sdk::signer::keypair::Keypair::new();

        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            charlie.pubkey(),
            &charlie,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "charlie".to_string(),
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

        // Now create a new user with referrer
        let david = solana_sdk::signer::keypair::Keypair::new();
        let charlie_profile = utils::pda::get_user_profile_pda(&charlie.pubkey()).0;

        test_instructions::init_user_profile(
            &test_setup.program_test_ctx,
            david.pubkey(),
            &david,
            &test_setup.payer_keypair,
            InitUserProfileParams {
                nickname: "david".to_string(),
                profile_picture: ProfilePicture::Zero as u8,
                wallpaper: Wallpaper::Zero as u8,
                title: Title::Zero as u8,
                team: Team::Default as u8,
                continent: Continent::Default as u8,
            },
            Some(charlie_profile),
        )
        .await
        .unwrap();

        // Verify the referrer was set
        let david_profile_pda = utils::pda::get_user_profile_pda(&david.pubkey()).0;
        let david_profile_account = utils::get_zero_copy_account::<UserProfile>(
            &test_setup.program_test_ctx,
            david_profile_pda,
        )
        .await;

        assert_eq!(david_profile_account.referrer_profile, charlie_profile);
    }
}
