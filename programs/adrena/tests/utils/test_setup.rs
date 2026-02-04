use {
    super::{initialize_token_accounts, pda, ChaosLabsFeedIdEnum, SetupCustodyInfo},
    crate::{
        adapters, test_instructions,
        utils::{self, fixtures},
    },
    adrena::{
        adapters::{mpl_token_metadata_program_adapter, spl_governance_program_adapter},
        instructions::{
            AddCustodyParams, AddLiquidityParams, InitOneParams, InitOracleParams,
            InitStakingOneParams, OraclePricesSetup,
        },
        state::{
            cortex::Cortex,
            custody::{BorrowRateParams, Fees, PricingParams},
            oracle::{Oracle, OraclePrice, MAX_ORACLE_PRICES_COUNT},
            pool::{PoolLiquidityState, TokenRatios, MAX_CUSTODIES},
            staking::StakingType,
        },
        utils::limited_string::LimitedString,
    },
    anchor_spl::associated_token::get_associated_token_address,
    solana_program::pubkey::Pubkey,
    solana_program_test::{processor, ProgramTest, ProgramTestContext},
    solana_sdk::{clock::Clock, signature::Keypair, signer::Signer},
    std::collections::HashMap,
    tokio::sync::RwLock,
};

pub struct SetupCustodyWithLiquidityParams<'a> {
    pub setup_custody_params: SetupCustodyParams<'a>,
    //
    // /!\ Warning: Ignored if genesis liquidity is not skipped
    //
    pub liquidity_amount: u64,
    // Who's adding the liquidity?
    pub payer_user_name: &'a str,
}

pub struct SetupCustodyOracleParam {
    pub initial_price: u64,
    pub initial_conf: u64,
    pub oracle_name: LimitedString,
    pub chaos_labs_feed_id: ChaosLabsFeedIdEnum,
}

pub struct SetupCustodyParams<'a> {
    // Which mint is it about
    pub mint_name: &'a str,

    pub is_stable: bool,
    pub target_ratio: u16,
    pub min_ratio: u16,
    pub max_ratio: u16,
    pub pricing_params: Option<PricingParams>,
    pub fees: Option<Fees>,
    pub borrow_rate: Option<BorrowRateParams>,
    pub oracle: SetupCustodyOracleParam,
    // If true, the trade_oracle will be different than the oracle (i.e jitoSOL oracle and SOL trade oracle)
    pub trade_oracle: Option<SetupCustodyOracleParam>,
}

pub struct UserParam<'a> {
    pub name: &'a str,

    // mint_name: amount
    pub token_balances: HashMap<&'a str, u64>,
}

pub struct MintParam<'a> {
    pub name: &'a str,
    pub decimals: u8,
}

pub struct MintInfo {
    pub decimals: u8,
    pub pubkey: Pubkey,
}

pub struct TestSetup {
    pub program_test_ctx: RwLock<ProgramTestContext>,

    pub root_authority_keypair: Keypair,
    pub admin_keypair: Keypair,
    pub payer_keypair: Keypair,

    pub users: HashMap<String, Keypair>,
    pub mints: HashMap<String, MintInfo>,

    pub fee_redistribution_mint_name: String,

    pub governance_realm_pda: Pubkey,
    pub gov_token_mint_pda: Pubkey,
    pub lm_token_mint: Pubkey,
    pub protocol_fee_recipient_usdc_token_account: Pubkey,

    pub pool_pda: Pubkey,
    pub pool_bump: u8,
    pub lp_token_mint_pda: Pubkey,
    pub lp_token_mint_bump: u8,
    pub custodies_info: Vec<SetupCustodyInfo>,
}

impl TestSetup {
    pub fn get_user_keypair_by_name(&self, name: &str) -> &Keypair {
        self.users.get(&name.to_string()).unwrap()
    }

    pub fn get_fee_redistribution_mint(&self) -> Pubkey {
        self.mints
            .get(&self.fee_redistribution_mint_name)
            .unwrap()
            .pubkey
    }

    pub fn get_mint_by_name(&self, name: &str) -> Pubkey {
        self.mints.get(&name.to_string()).unwrap().pubkey
    }

    // Initialize everything required to test the program
    // Create the mints, the users, deploy the program, create the pool and the custodies, provide liquidity.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        users_param: Vec<UserParam<'_>>,
        mints_param: Vec<MintParam<'_>>,
        // mint for the payouts of the LM token staking (ADX staking)
        fee_redistribution_mint_name: &str,
        governance_token_decimals: u8,
        governance_realm_name: &str,
        pool_name: &str,
        aum_soft_cap_usd: u64,
        custodies_params: Vec<SetupCustodyWithLiquidityParams<'_>>,
        core_contributor_bucket_allocation: u64,
        foundation_bucket_allocation: u64,
        ecosystem_bucket_allocation: u64,
        // If provided, will start everything at a specific timestamp in the future
        // Useful for tests that require a specific date to be tested
        start_timestamp: Option<i64>,
        whitelisted_swapper: Option<&str>, // Name of the whitelisted user
    ) -> TestSetup {
        // Mute logs for the setup, as they not interesting logs
        // utils::mute_logs();

        let mut program_test = ProgramTest::new("adrena", adrena::id(), None);

        // Initialize keypairs
        let keypairs: Vec<Keypair> = utils::create_and_fund_multiple_accounts(
            &mut program_test,
            // 1 keypair per user
            users_param.len() +
            // payer
            1 +
            // root authority
            1 +
            // admin
            1 +
            // protocol fee recipient (dao account for instance, for buybacks)
            1,
        )
        .await;

        // Name keypairs
        let (
            users_keypairs,
            payer_keypair,
            root_authority_keypair,
            admin_keypair,
            protocol_fee_recipient_keypair,
        ) = {
            (
                &keypairs[0..users_param.len()],
                keypairs.get(users_param.len()).unwrap(),
                keypairs.get(users_param.len() + 1).unwrap(),
                keypairs.get(users_param.len() + 2).unwrap(),
                keypairs.get(users_param.len() + 3).unwrap(),
            )
        };

        let users = {
            let mut users: HashMap<String, Keypair> = HashMap::new();
            for (i, user_param) in users_param.as_slice().iter().enumerate() {
                users.insert(
                    user_param.name.to_string(),
                    users_keypairs[i].insecure_clone(),
                );
            }

            users
        };

        // Initialize mints
        let mints = {
            let mut mints: HashMap<String, MintInfo> = HashMap::new();

            for mint_param in mints_param {
                let mint_pubkey = utils::add_mint(
                    &mut program_test,
                    None,
                    mint_param.decimals,
                    &root_authority_keypair.pubkey(),
                )
                .await
                .unwrap()
                .0;

                mints.insert(
                    mint_param.name.to_string(),
                    MintInfo {
                        decimals: mint_param.decimals,
                        pubkey: mint_pubkey,
                    },
                );
            }

            mints
        };

        // Deploy programs
        {
            program_test.add_program("adrena", adrena::ID, processor!(adrena::entry));

            program_test.add_program(
                "spl_governance",
                spl_governance_program_adapter::id(),
                processor!(spl_governance::processor::process_instruction),
            );

            program_test.add_program(
                "mpl_token_metadata",
                mpl_token_metadata_program_adapter::id(),
                None,
            );
        }

        // Start the client and connect to localnet validator
        let program_test_ctx: RwLock<ProgramTestContext> =
            RwLock::new(program_test.start_with_context().await);

        let fee_redistribution_mint = &mints.get(fee_redistribution_mint_name).unwrap().pubkey;

        let governance_realm_pda = pda::get_governance_realm_pda(governance_realm_name.to_string());
        let gov_token_mint_pda = pda::get_governance_token_mint_pda().0;

        let mut clock_sysvar: Clock = program_test_ctx
            .write()
            .await
            .banks_client
            .get_sysvar()
            .await
            .unwrap();

        // Warp in the future, to the exact date user asked for
        if let Some(start_timestamp) = start_timestamp {
            let current_time = utils::get_current_unix_timestamp(&program_test_ctx).await;

            let seconds_until_start_timestamp = start_timestamp - current_time;

            if seconds_until_start_timestamp < 0 {
                panic!("start_timestamp must be in the future (may be time to update tests)");
            }

            utils::warp_forward(&program_test_ctx, seconds_until_start_timestamp).await;

            clock_sysvar = program_test_ctx
                .write()
                .await
                .banks_client
                .get_sysvar()
                .await
                .unwrap();
        }

        // Execute the initialize transaction
        initialize_token_accounts(
            &program_test_ctx,
            mints["usdc"].pubkey,
            payer_keypair,
            &[protocol_fee_recipient_keypair.pubkey()],
        )
        .await
        .unwrap();

        let protocol_fee_recipient_usdc_token_account = get_associated_token_address(
            &protocol_fee_recipient_keypair.pubkey(),
            &mints["usdc"].pubkey,
        );

        let lm_token_mint = utils::pda::get_lm_token_mint_pda().0;

        // Initialize the program
        {
            test_instructions::init_one_core(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                &protocol_fee_recipient_usdc_token_account,
                fee_redistribution_mint,
                InitOneParams {
                    core_contributor_bucket_allocation,
                    foundation_bucket_allocation,
                    ecosystem_bucket_allocation,
                },
            )
            .await
            .unwrap();

            test_instructions::init_two_lm_token_metadata(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
            )
            .await
            .unwrap();

            test_instructions::init_three_governance(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                &governance_realm_pda,
            )
            .await
            .unwrap();

            test_instructions::init_four_vesting(&program_test_ctx, admin_keypair, payer_keypair)
                .await
                .unwrap();
        }

        // Initialize the LM staking
        {
            test_instructions::init_staking_one(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lm_token_mint,
                &InitStakingOneParams {
                    staking_type: StakingType::LM.into(),
                },
            )
            .await
            .unwrap();

            test_instructions::init_staking_two(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lm_token_mint,
            )
            .await
            .unwrap();

            test_instructions::init_staking_three(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lm_token_mint,
            )
            .await
            .unwrap();

            test_instructions::init_staking_four(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lm_token_mint,
            )
            .await
            .unwrap();
        }

        // Setup the governance
        {
            adapters::spl_governance::create_realm(
                &program_test_ctx,
                root_authority_keypair,
                payer_keypair,
                governance_realm_name.to_string(),
                utils::scale(10_000, governance_token_decimals),
                &gov_token_mint_pda,
            )
            .await
            .unwrap();
        }

        // Initialize users token accounts for each mints
        {
            let mut mints_pubkeys: Vec<Pubkey> = mints.values().map(|info| info.pubkey).collect();

            mints_pubkeys.push(lm_token_mint);

            let mut users_pubkeys: Vec<Pubkey> =
                users.values().map(|keypair| keypair.pubkey()).collect();

            // Init also for admin as we use it's ATA sometimes to mimic DAO permissioned ix
            users_pubkeys.push(admin_keypair.pubkey());

            utils::initialize_users_token_accounts(
                &program_test_ctx,
                payer_keypair,
                mints_pubkeys,
                users_pubkeys,
            )
            .await;
        }

        // Mint tokens for users to match specified balances
        {
            for user_param in users_param.as_slice() {
                for (mint_name, amount) in &user_param.token_balances {
                    let mint = mints.get(&mint_name.to_string()).unwrap().pubkey;
                    let user = users.get(&user_param.name.to_string()).unwrap().pubkey();

                    let ata = utils::find_associated_token_account(&user, &mint).0;

                    utils::mint_tokens(
                        &program_test_ctx,
                        root_authority_keypair,
                        &mint,
                        &ata,
                        *amount,
                    )
                    .await;
                }
            }
        }

        // Lasts 3 days
        let genesis_lock_campaign_duration = 84600 * 3;
        // Starts 600 seconds after init
        let genesis_lock_campaign_start_date = clock_sysvar.unix_timestamp + 600;
        // Ends one day after start
        let genesis_reserved_grant_duration = 84600;

        // Setup the pool
        let (pool_pda, pool_bump, lp_token_mint_pda, lp_token_mint_bump) =
            test_instructions::add_pool_part_one(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                pool_name,
                aum_soft_cap_usd,
            )
            .await
            .unwrap();

        // We can generate random reserved spots, as they won't be used
        let reserved_spot_a: Pubkey = Pubkey::new_unique();
        let reserved_spot_b = Pubkey::new_unique();

        test_instructions::add_pool_part_two(
            &program_test_ctx,
            admin_keypair,
            payer_keypair,
            pool_name,
            genesis_lock_campaign_duration,
            genesis_lock_campaign_start_date,
            genesis_reserved_grant_duration,
            &reserved_spot_a,
            &reserved_spot_b,
        )
        .await
        .unwrap();

        if let Some(whitelisted_swapper) = whitelisted_swapper {
            let user = users
                .get(&whitelisted_swapper.to_string())
                .unwrap()
                .pubkey();

            test_instructions::set_pool_whitelisted_swapper(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                &pool_pda,
                &user,
            )
            .await
            .unwrap();
        }

        // Initialize the oracle account
        {
            let mut v: Vec<OraclePricesSetup> = Vec::new();

            for custody_param in custodies_params.iter() {
                v.push(OraclePricesSetup {
                    name: custody_param.setup_custody_params.oracle.oracle_name,
                    chaos_labs_feed_id: custody_param
                        .setup_custody_params
                        .oracle
                        .chaos_labs_feed_id
                        .to_u8(),
                });
            }

            test_instructions::init_oracle(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                &InitOracleParams { oracle_prices: v },
            )
            .await
            .unwrap();
        }

        // Setup the custodies
        // Do it without ratio bound so we can provide liquidity without ratio limit error
        let custodies_info: Vec<SetupCustodyInfo> = {
            let mut custodies_info: Vec<SetupCustodyInfo> = Vec::new();

            let mut ratios: [TokenRatios; MAX_CUSTODIES] = Default::default();

            for (idx, custody_param) in custodies_params.iter().enumerate() {
                let mint_info = mints
                    .get(&custody_param.setup_custody_params.mint_name.to_string())
                    .unwrap();

                // Spread the target ratio to match exactly 100 with all custodies taken into account
                let target_ratio = 10_000 / (idx + 1) as u16;

                // Force ratio 0 to 100% to be able to provide liquidity
                ratios[idx] = TokenRatios {
                    target: target_ratio,
                    min: 0,
                    max: 10_000,
                    ..TokenRatios::default()
                };

                // Apply new ratios to all custodies
                {
                    let mut index: usize = 0;

                    ratios.iter_mut().for_each(|x| {
                        if index < idx {
                            x.target = target_ratio
                        }

                        index += 1;
                    });
                }

                // In case of a remainder, add it to the last custody
                if 10000 % (idx + 1) != 0 {
                    ratios[idx].target += 10_000 % (idx + 1) as u16;
                }

                let custody_pda = {
                    let add_custody_params = AddCustodyParams {
                        is_stable: custody_param.setup_custody_params.is_stable,
                        pricing: custody_param
                            .setup_custody_params
                            .pricing_params
                            .unwrap_or_else(fixtures::pricing_params_regular),
                        fees: custody_param
                            .setup_custody_params
                            .fees
                            .unwrap_or_else(fixtures::fees_regular),
                        borrow_rate: custody_param
                            .setup_custody_params
                            .borrow_rate
                            .unwrap_or_else(fixtures::borrow_rate_regular),
                        oracle: custody_param.setup_custody_params.oracle.oracle_name,
                        trade_oracle: match &custody_param.setup_custody_params.trade_oracle {
                            Some(trade_oracle) => trade_oracle.oracle_name,
                            None => custody_param.setup_custody_params.oracle.oracle_name,
                        },

                        // in BPS, 10_000 = 100%
                        ratios,
                        allow_swap: true,
                        allow_trade: true,
                    };

                    test_instructions::add_custody(
                        &program_test_ctx,
                        admin_keypair,
                        payer_keypair,
                        &pool_pda,
                        &mint_info.pubkey,
                        mint_info.decimals,
                        add_custody_params,
                    )
                    .await
                    .unwrap()
                    .0
                };

                utils::warp_forward(&program_test_ctx, 1).await;

                custodies_info.push(SetupCustodyInfo {
                    oracle: custody_param.setup_custody_params.oracle.oracle_name,
                    trade_oracle: match &custody_param.setup_custody_params.trade_oracle {
                        Some(trade_oracle) => trade_oracle.oracle_name,
                        None => custody_param.setup_custody_params.oracle.oracle_name,
                    },
                    mint: mint_info.pubkey,
                    custody_pda,
                });
            }

            custodies_info
        };

        // Push oracle prices
        {
            let publish_time = utils::get_current_unix_timestamp(&program_test_ctx).await;

            let mut oracle_prices: Vec<OraclePrice> = vec![];

            for custody_param in custodies_params.iter() {
                oracle_prices.push(OraclePrice {
                    price: custody_param.setup_custody_params.oracle.initial_price,
                    confidence: custody_param.setup_custody_params.oracle.initial_conf,
                    timestamp: publish_time,
                    exponent: -(Cortex::PRICE_DECIMALS as i32),
                    name: custody_param.setup_custody_params.oracle.oracle_name,
                    chaos_labs_feed_id: custody_param
                        .setup_custody_params
                        .oracle
                        .chaos_labs_feed_id
                        .to_u8(),
                    _padding: Default::default(),
                });

                if custody_param.setup_custody_params.trade_oracle.is_some() {
                    let trade_oracle_param = custody_param
                        .setup_custody_params
                        .trade_oracle
                        .as_ref()
                        .unwrap();

                    oracle_prices.push(OraclePrice {
                        price: trade_oracle_param.initial_price,
                        confidence: trade_oracle_param.initial_conf,
                        timestamp: publish_time,
                        exponent: -(Cortex::PRICE_DECIMALS as i32),
                        name: trade_oracle_param.oracle_name,
                        chaos_labs_feed_id: trade_oracle_param.chaos_labs_feed_id.to_u8(),
                        _padding: Default::default(),
                    });
                }
            }

            while oracle_prices.len() < MAX_ORACLE_PRICES_COUNT {
                oracle_prices.push(OraclePrice::default());
            }

            // Override the oracle
            {
                let oracle_pda = pda::get_oracle_pda().0;
                let oracle = utils::get_account::<Oracle>(&program_test_ctx, oracle_pda).await;

                utils::write_oracle(
                    &program_test_ctx,
                    Oracle {
                        bump: oracle.bump,
                        _padding: Default::default(),
                        updated_at: publish_time,
                        prices: oracle_prices.try_into().unwrap(),
                    },
                )
                .await;
            }
        }

        // Initialize LP staking
        {
            test_instructions::init_staking_one(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lp_token_mint_pda,
                &InitStakingOneParams {
                    staking_type: StakingType::LP.into(),
                },
            )
            .await
            .unwrap();

            test_instructions::init_staking_two(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lp_token_mint_pda,
            )
            .await
            .unwrap();

            test_instructions::init_staking_three(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lp_token_mint_pda,
            )
            .await
            .unwrap();

            test_instructions::init_staking_four(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                fee_redistribution_mint,
                &lp_token_mint_pda,
            )
            .await
            .unwrap();
        }

        utils::warp_forward(&program_test_ctx, 1).await;

        // Initialize users token accounts for lp token mint
        {
            let users_pubkeys: Vec<Pubkey> =
                users.values().map(|keypair| keypair.pubkey()).collect();

            utils::initialize_users_token_accounts(
                &program_test_ctx,
                payer_keypair,
                vec![lp_token_mint_pda],
                users_pubkeys,
            )
            .await;
        }

        // Jump to the start of the campaign
        utils::warp_forward(
            &program_test_ctx,
            genesis_lock_campaign_start_date - clock_sysvar.unix_timestamp,
        )
        .await;

        // Jump to the end of the campaign
        utils::warp_forward(&program_test_ctx, genesis_lock_campaign_duration).await;

        // Trigger the end of the campaign
        utils::execute_finalize_genesis_lock_campaign_automation(
            &program_test_ctx,
            payer_keypair,
            &pool_pda,
        )
        .await
        .unwrap();

        utils::warp_forward(&program_test_ctx, 1).await;

        // Change pool state
        test_instructions::set_pool_liquidity_state(
            &program_test_ctx,
            admin_keypair,
            payer_keypair,
            &pool_pda,
            PoolLiquidityState::Active,
        )
        .await
        .unwrap();

        // Allow pool swaps and pool trades
        test_instructions::set_pool_allow_swap(
            &program_test_ctx,
            admin_keypair,
            payer_keypair,
            &pool_pda,
            true,
        )
        .await
        .unwrap();

        test_instructions::set_pool_allow_trade(
            &program_test_ctx,
            admin_keypair,
            payer_keypair,
            &pool_pda,
            true,
        )
        .await
        .unwrap();

        utils::warp_forward(&program_test_ctx, 1).await;

        // Add liquidity
        for custody_param in custodies_params.as_slice() {
            let mint_info = mints
                .get(&custody_param.setup_custody_params.mint_name.to_string())
                .unwrap();

            let liquidity_provider = users
                .get(&custody_param.payer_user_name.to_string())
                .unwrap();

            println!(
                "adding liquidity for mint {}",
                custody_param.setup_custody_params.mint_name
            );

            if custody_param.liquidity_amount > 0 {
                test_instructions::add_liquidity(
                    &program_test_ctx,
                    liquidity_provider,
                    payer_keypair,
                    &pool_pda,
                    &mint_info.pubkey,
                    AddLiquidityParams {
                        amount_in: custody_param.liquidity_amount,
                        min_lp_amount_out: 1,
                        oracle_prices: Some(
                            utils::get_oracle_prices_as_chaos_labs_bundle(&program_test_ctx).await,
                        ),
                    },
                )
                .await
                .unwrap();
            }
        }

        // Set proper ratios for custodies
        {
            let target_ratio = 10_000 / custodies_params.len() as u16;

            let mut ratios: [TokenRatios; MAX_CUSTODIES] = Default::default();

            let mut index: usize = 0;

            custodies_params.iter().for_each(|x| {
                ratios[index] = TokenRatios {
                    target: target_ratio,
                    min: x.setup_custody_params.min_ratio,
                    max: x.setup_custody_params.max_ratio,
                    ..TokenRatios::default()
                };

                index += 1;
            });

            // If there is a remainder, add it to the last ratio
            if 10_000 % custodies_params.len() != 0 {
                ratios[custodies_params.len() - 1].target += 10_000 % custodies_params.len() as u16;
            }

            for (idx, _params) in custodies_params.as_slice().iter().enumerate() {
                utils::set_custody_ratios(
                    &program_test_ctx,
                    admin_keypair,
                    payer_keypair,
                    &custodies_info[idx].custody_pda,
                    ratios,
                )
                .await;
            }
        }

        {
            test_instructions::disable_tokens_freeze_capabilities(
                &program_test_ctx,
                admin_keypair,
                payer_keypair,
                &pool_pda,
            )
            .await
            .unwrap();
        }

        utils::unmute_logs();

        TestSetup {
            program_test_ctx,
            root_authority_keypair: root_authority_keypair.insecure_clone(),
            admin_keypair: admin_keypair.insecure_clone(),
            payer_keypair: payer_keypair.insecure_clone(),
            users,
            mints,
            fee_redistribution_mint_name: fee_redistribution_mint_name.to_string(),
            governance_realm_pda,
            gov_token_mint_pda,
            lm_token_mint,
            protocol_fee_recipient_usdc_token_account,
            pool_pda,
            pool_bump,
            lp_token_mint_pda,
            lp_token_mint_bump,
            custodies_info,
        }
    }

    // Use the mint_name as anchor to ease the call to the function
    pub async fn update_oracle_price(&self, mint_name: &str, price: u64, conf: u64) {
        let publish_time: i64 = utils::get_current_unix_timestamp(&self.program_test_ctx).await;

        let oracle_pda = pda::get_oracle_pda().0;

        let mut oracle = utils::get_account::<Oracle>(&self.program_test_ctx, oracle_pda).await;

        let name = LimitedString::new(mint_name);

        match oracle.prices.iter_mut().find(|x| x.name.eq(&name)) {
            Some(p) => {
                // Mutate existing price
                p.price = price;
                p.confidence = conf;
                p.timestamp = publish_time;
            }
            None => {
                // Add a new price
                match oracle.prices.iter_mut().find(|x| x.name.length == 0) {
                    Some(p) => {
                        p.price = price;
                        p.confidence = conf;
                        p.timestamp = publish_time;
                        p.exponent = Cortex::PRICE_DECIMALS as i32;
                        p.name = name;
                        p.chaos_labs_feed_id = Default::default(); // TODO: set something
                    }
                    None => {
                        panic!("No empty slot found in oracle prices");
                    }
                }
            }
        }

        utils::write_oracle(&self.program_test_ctx, oracle).await;
    }
}
