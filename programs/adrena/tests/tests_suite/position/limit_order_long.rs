use {
    crate::{
        test_instructions,
        utils::{self, ChaosLabsFeedIdEnum, SetupCustodyOracleParam},
    },
    adrena::{
        instructions::{
            AddLimitOrderParams, CancelLimitOrderParams, ClosePositionLongParams,
            ExecuteLimitOrderLongParams, InitUserProfileParams,
        },
        state::{
            cortex::Cortex,
            limit_order_book::LimitOrderBook,
            position::{Position, Side},
            user_profile::{Continent, ProfilePicture, Team, Title, Wallpaper},
        },
        utils::limited_string::LimitedString,
    },
    anchor_spl::token::TokenAccount,
    maplit::hashmap,
    solana_sdk::{rent::Rent, signer::Signer},
};

const JITOSOL_DECIMALS: u8 = 9;
const USDC_DECIMALS: u8 = 6;

pub async fn limit_order_long() {
    let test_setup = utils::TestSetup::new(
        vec![
            utils::UserParam {
                name: "alice",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(500, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "martin",
                token_balances: hashmap! {
                    "usdc" => utils::scale(150_000, USDC_DECIMALS),
                    "jitoSOL" => utils::scale(100, JITOSOL_DECIMALS),
                },
            },
            utils::UserParam {
                name: "executioner",
                token_balances: hashmap! {},
            },
        ],
        vec![
            utils::MintParam {
                name: "usdc",
                decimals: USDC_DECIMALS,
            },
            utils::MintParam {
                name: "jitoSOL",
                decimals: JITOSOL_DECIMALS,
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
                liquidity_amount: utils::scale(150_000, USDC_DECIMALS),
                payer_user_name: "alice",
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint_name: "jitoSOL",
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    oracle: SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("jitoSOL"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::JITOSOL,
                    },
                    trade_oracle: Some(SetupCustodyOracleParam {
                        initial_price: utils::scale(100, Cortex::PRICE_DECIMALS),
                        initial_conf: 0,
                        oracle_name: LimitedString::new("sol"),
                        chaos_labs_feed_id: ChaosLabsFeedIdEnum::SOL,
                    }),
                    pricing_params: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(100, JITOSOL_DECIMALS),
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

    let martin = test_setup.get_user_keypair_by_name("martin");
    let alice = test_setup.get_user_keypair_by_name("alice");

    let jitosol_mint = &test_setup.get_mint_by_name("jitoSOL");

    let alice_jitosol_ata = utils::find_associated_token_account(&alice.pubkey(), jitosol_mint).0;

    {
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
    }

    let martin_lamports_a =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    test_instructions::init_limit_order_book(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    let martin_lamports_b =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Pay for order book PDA rent exempt$
    assert_eq!(
        martin_lamports_b as i128 - martin_lamports_a as i128,
        -(Rent::default().minimum_balance(LimitOrderBook::LEN) as i128)
    );

    test_instructions::add_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        jitosol_mint,
        AddLimitOrderParams {
            leverage: 50000,
            trigger_price: utils::scale(90, Cortex::PRICE_DECIMALS),
            limit_price: None,
            side: Side::Long.into(),
            amount: utils::scale(5, JITOSOL_DECIMALS),
        },
    )
    .await
    .unwrap();

    let martin_lamports_c =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Pay for accounts rent exempt + Automation fee + user profile setup
    assert_eq!(
        martin_lamports_c as i128 - martin_lamports_b as i128,
        -((Rent::default().minimum_balance(TokenAccount::LEN)
            + Rent::default().minimum_balance(Position::LEN)
            + Cortex::AUTOMATION_EXECUTION_FEE) as i128)
    );

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Wrong id should fail
    assert!(test_instructions::cancel_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        CancelLimitOrderParams { id: 2 },
    )
    .await
    .is_err());

    assert!(test_instructions::cancel_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        CancelLimitOrderParams { id: 0 },
    )
    .await
    .is_err());

    // cancel then re-add (to test cancel works)
    test_instructions::cancel_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        CancelLimitOrderParams { id: 1 },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let martin_lamports_d =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Get back automation fee + accounts rent
    assert_eq!(
        martin_lamports_d - martin_lamports_c,
        Rent::default().minimum_balance(LimitOrderBook::LEN)
            + Rent::default().minimum_balance(TokenAccount::LEN)
            + Rent::default().minimum_balance(Position::LEN)
            + Cortex::AUTOMATION_EXECUTION_FEE
    );

    test_instructions::init_limit_order_book(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    let martin_lamports_e =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Pay for order book PDA rent exempt
    assert_eq!(
        martin_lamports_e as i128 - martin_lamports_d as i128,
        -(Rent::default().minimum_balance(LimitOrderBook::LEN) as i128)
    );

    test_instructions::add_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        jitosol_mint,
        AddLimitOrderParams {
            leverage: 50000,
            trigger_price: utils::scale(90, Cortex::PRICE_DECIMALS),
            limit_price: None,
            side: Side::Long.into(),
            amount: utils::scale(5, JITOSOL_DECIMALS),
        },
    )
    .await
    .unwrap();

    let martin_lamports_f =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Pay for accounts rent exempt + Automation fee
    assert_eq!(
        martin_lamports_f as i128 - martin_lamports_e as i128,
        -((Rent::default().minimum_balance(TokenAccount::LEN)
            + Rent::default().minimum_balance(Position::LEN)
            + Cortex::AUTOMATION_EXECUTION_FEE) as i128)
    );

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let alice_lamports_before =
        utils::get_lamports(&test_setup.program_test_ctx, alice.pubkey()).await;

    // Price not reached should fail
    assert!(test_instructions::execute_limit_order_long(
        &test_setup.program_test_ctx,
        alice,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        ExecuteLimitOrderLongParams {
            id: 2,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());

    // Wrong ID should fail
    assert!(test_instructions::execute_limit_order_long(
        &test_setup.program_test_ctx,
        alice,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        ExecuteLimitOrderLongParams {
            id: 2,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .is_err());

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Move price to trigger limit order
    test_setup
        .update_oracle_price("sol", utils::scale(89, Cortex::PRICE_DECIMALS), 0)
        .await;

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let position_pda = test_instructions::execute_limit_order_long(
        &test_setup.program_test_ctx,
        alice,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        ExecuteLimitOrderLongParams {
            id: 2,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap()
    .0;

    let alice_lamports_after =
        utils::get_lamports(&test_setup.program_test_ctx, alice.pubkey()).await;

    let martin_lamports_g =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Get back account rent
    assert_eq!(
        martin_lamports_g - martin_lamports_f,
        Rent::default().minimum_balance(LimitOrderBook::LEN)
            + Rent::default().minimum_balance(TokenAccount::LEN)
    );

    // Executor should receive execution fee
    assert_eq!(
        alice_lamports_after - alice_lamports_before,
        Cortex::AUTOMATION_EXECUTION_FEE
    );

    utils::warp_forward(&test_setup.program_test_ctx, 11).await;

    test_instructions::init_limit_order_book(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
    )
    .await
    .unwrap();

    let martin_lamports_h =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    assert_eq!(
        martin_lamports_h as i128 - martin_lamports_g as i128,
        -(Rent::default().minimum_balance(LimitOrderBook::LEN) as i128)
    );

    // Add a new limit order that should trigger when price goes down a bit -> should trigger an increase_position
    test_instructions::add_limit_order(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        jitosol_mint,
        AddLimitOrderParams {
            leverage: 50000,
            trigger_price: utils::scale(85, Cortex::PRICE_DECIMALS),
            limit_price: Some(utils::scale(80, Cortex::PRICE_DECIMALS)),
            side: Side::Long.into(),
            amount: utils::scale(5, JITOSOL_DECIMALS),
        },
    )
    .await
    .unwrap();

    let martin_lamports_i =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    assert_eq!(
        martin_lamports_i as i128 - martin_lamports_h as i128,
        -((Rent::default().minimum_balance(TokenAccount::LEN)
            + Rent::default().minimum_balance(Position::LEN)
            + Cortex::AUTOMATION_EXECUTION_FEE) as i128)
    );

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    // Move price to trigger limit order
    test_setup
        .update_oracle_price("sol", utils::scale(82, Cortex::PRICE_DECIMALS), 0)
        .await;

    // Send some collateral to the escrow account to simulate a user trying to game the system and block user rent
    {
        let collateral_escrow_pda = utils::pda::get_collateral_escrow_pda(
            &test_setup.pool_pda,
            &martin.pubkey(),
            jitosol_mint,
        )
        .0;

        utils::transfer_tokens(
            &test_setup.program_test_ctx,
            &test_setup.payer_keypair,
            alice,
            &alice_jitosol_ata,
            &collateral_escrow_pda,
            1_000_000,
        )
        .await
        .unwrap();
    }

    // Should be an increase
    test_instructions::execute_limit_order_long(
        &test_setup.program_test_ctx,
        alice,
        martin,
        &test_setup.payer_keypair,
        &test_setup.pool_pda,
        jitosol_mint,
        ExecuteLimitOrderLongParams {
            id: 3,
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
        },
    )
    .await
    .unwrap();

    utils::warp_forward(&test_setup.program_test_ctx, 1).await;

    let martin_lamports_j =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Get back rents for closed accounts
    assert_eq!(
        martin_lamports_j - martin_lamports_i,
        Rent::default().minimum_balance(LimitOrderBook::LEN)
            + Rent::default().minimum_balance(TokenAccount::LEN)
            + Rent::default().minimum_balance(Position::LEN) // because already opened
    );

    test_instructions::close_position_long(
        &test_setup.program_test_ctx,
        martin,
        &test_setup.payer_keypair,
        &position_pda,
        ClosePositionLongParams {
            price: Some(utils::scale(80, Cortex::PRICE_DECIMALS)),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(&test_setup.program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    .unwrap();

    let martin_lamports_k =
        utils::get_lamports(&test_setup.program_test_ctx, martin.pubkey()).await;

    // Get back position rent
    assert_eq!(
        martin_lamports_k - martin_lamports_j,
        Rent::default().minimum_balance(Position::LEN),
    );
}
