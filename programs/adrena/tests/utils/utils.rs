use {
    super::pda,
    crate::{test_instructions, InstructionInfo, INSTRUCTION_LOG},
    adrena::{
        adapters::SplGovernanceV3Adapter,
        instructions::SetCustodyConfigParams,
        program::Adrena,
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices,
            cortex::Cortex,
            custody::Custody,
            oracle::Oracle,
            pool::{Pool, TokenRatios, MAX_CUSTODIES},
        },
        utils::limited_string::LimitedString,
    },
    anchor_lang::{prelude::*, Discriminator, InstructionData},
    anchor_spl::token::spl_token::{self, state::Mint},
    bincode::serialize,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        clock::DEFAULT_MS_PER_SLOT, epoch_schedule::DEFAULT_SLOTS_PER_EPOCH, program_pack::Pack,
        stake_history::Epoch,
    },
    solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext},
    solana_sdk::{
        account::{self, AccountSharedData, ReadableAccount},
        borsh0_10,
        compute_budget::ComputeBudgetInstruction,
        instruction::Instruction,
        signature::Keypair,
        signer::Signer,
        signers::Signers,
        transaction::Transaction,
    },
    std::{
        alloc::Layout,
        ops::{Div, Mul},
    },
    tokio::sync::RwLock,
};

pub const ANCHOR_DISCRIMINATOR_SIZE: usize = 8;

#[repr(u8)]
pub enum ChaosLabsFeedIdEnum {
    SOL = 0,
    JITOSOL = 1,
    BTC = 2,
    WBTC = 3,
    BONK = 4,
    USDC = 5,
    ETH = 6,
}

impl ChaosLabsFeedIdEnum {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ChaosLabsFeedIdEnum::SOL),
            1 => Some(ChaosLabsFeedIdEnum::JITOSOL),
            2 => Some(ChaosLabsFeedIdEnum::BTC),
            3 => Some(ChaosLabsFeedIdEnum::WBTC),
            4 => Some(ChaosLabsFeedIdEnum::BONK),
            5 => Some(ChaosLabsFeedIdEnum::USDC),
            6 => Some(ChaosLabsFeedIdEnum::ETH),
            _ => None,
        }
    }

    pub const fn to_u8(&self) -> u8 {
        match self {
            ChaosLabsFeedIdEnum::SOL => 0,
            ChaosLabsFeedIdEnum::JITOSOL => 1,
            ChaosLabsFeedIdEnum::BTC => 2,
            ChaosLabsFeedIdEnum::WBTC => 3,
            ChaosLabsFeedIdEnum::BONK => 4,
            ChaosLabsFeedIdEnum::USDC => 5,
            ChaosLabsFeedIdEnum::ETH => 6,
        }
    }
}

#[macro_export]
macro_rules! assert_unchanged {
    ($before:expr, $after:expr) => {
        assert_eq!(
            $before, $after,
            "Values are not the same: {:?} != {:?}",
            $before, $after
        );
    };
}

pub fn create_and_fund_account(address: &Pubkey, program_test: &mut ProgramTest) {
    program_test.add_account(
        *address,
        account::Account {
            lamports: 21_000_000_000,
            ..account::Account::default()
        },
    );
}

pub fn find_associated_token_account(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            anchor_spl::token::ID.as_ref(),
            mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
}

pub fn days_in_seconds(nb_days: u32) -> i64 {
    (nb_days as i64) * (3_600 * 24)
}

pub async fn get_lamports(program_test_ctx: &RwLock<ProgramTestContext>, key: Pubkey) -> u64 {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let raw_account = banks_client.get_account(key).await.unwrap().unwrap();

    raw_account.lamports()
}

pub async fn get_token_account(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> spl_token::state::Account {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let raw_account = banks_client.get_account(key).await.unwrap().unwrap();

    spl_token::state::Account::unpack(&raw_account.data).unwrap()
}

pub async fn get_token_account_balance(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> u64 {
    get_token_account(program_test_ctx, key).await.amount
}

#[allow(deprecated)]
pub async fn get_borsh_account<T: BorshDeserialize>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    address: &Pubkey,
) -> T {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    banks_client
        .get_account(*address)
        .await
        .unwrap()
        .map(|a| borsh0_10::try_from_slice_unchecked(&a.data).unwrap())
        .unwrap_or_else(|| panic!("GET-TEST-ACCOUNT-ERROR: Account {} not found", address))
}

pub async fn try_get_account<T: anchor_lang::AccountDeserialize>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> Option<T> {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let account = banks_client.get_account(key).await.unwrap();

    // an account with 0 lamport can be considered inexistent in the context of our tests
    // on mainnet, someone might just send lamports to the right place but doesn't matter here
    return if let Some(a) = account {
        Some(T::try_deserialize(&mut a.data.as_slice()).unwrap())
    } else {
        None
    };
}

fn align_buffer(data: &[u8], align_to: usize) -> Vec<u8> {
    let layout = Layout::from_size_align(data.len(), align_to).expect("Invalid layout");
    unsafe {
        let aligned_buf = std::alloc::alloc(layout);
        std::ptr::copy_nonoverlapping(data.as_ptr(), aligned_buf, data.len());
        Vec::from_raw_parts(aligned_buf, data.len(), data.len())
    }
}

pub async fn try_get_zero_copy_account<T: bytemuck::Pod>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> Option<T> {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let account = banks_client.get_account(key).await.unwrap();

    // Have to have the data to be of the same alignment as the struct
    let data = &account.as_ref()?.data.clone()[8..];

    let aligned_data = align_buffer(data, std::mem::align_of::<T>());

    let tried = bytemuck::try_from_bytes::<T>(aligned_data.as_slice());

    if let Ok(tried) = tried {
        Some(*tried)
    } else {
        None
    }
}

pub async fn get_zero_copy_account<T: bytemuck::Pod>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> T {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let account = banks_client.get_account(key).await.unwrap().unwrap();

    // Have to have the data to be of the same alignment as the struct
    let data = &account.data.clone()[8..];

    let aligned_data = align_buffer(data, std::mem::align_of::<T>());

    *bytemuck::try_from_bytes::<T>(aligned_data.as_slice()).unwrap()
}

pub async fn get_account<T: anchor_lang::AccountDeserialize>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    key: Pubkey,
) -> T {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let account = banks_client.get_account(key).await.unwrap().unwrap();

    T::try_deserialize(&mut account.data.as_slice()).unwrap()
}

pub async fn get_current_unix_timestamp(program_test_ctx: &RwLock<ProgramTestContext>) -> i64 {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    banks_client
        .get_sysvar::<solana_program::sysvar::clock::Clock>()
        .await
        .unwrap()
        .unix_timestamp
}

pub async fn initialize_token_account(
    program_test_ctx: &RwLock<ProgramTestContext>,
    mint: &Pubkey,
    payer: &Keypair,
    owner: &Pubkey,
) -> Pubkey {
    initialize_token_accounts(program_test_ctx, *mint, payer, &[*owner])
        .await
        .unwrap()[0]
}

pub async fn mint_tokens(
    program_test_ctx: &RwLock<ProgramTestContext>,
    mint_authority: &Keypair,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
) {
    mint_tokens_internal(
        program_test_ctx,
        mint_authority,
        mint,
        token_account,
        amount,
    )
    .await
    .unwrap();
}

// Doesn't check if you go before epoch 0 when passing negative amounts, be wary
pub async fn warp_forward(ctx: &RwLock<ProgramTestContext>, seconds: i64) {
    let mut ctx = ctx.write().await;

    let clock_sysvar: Clock = ctx.banks_client.get_sysvar().await.unwrap();
    let mut new_clock = clock_sysvar.clone();
    new_clock.unix_timestamp += seconds;

    let seconds_since_epoch_start = new_clock.unix_timestamp - clock_sysvar.epoch_start_timestamp;
    let ms_since_epoch_start = seconds_since_epoch_start * 1_000;
    let slots_since_epoch_start = ms_since_epoch_start / DEFAULT_MS_PER_SLOT as i64;
    let epochs_since_epoch_start = slots_since_epoch_start / DEFAULT_SLOTS_PER_EPOCH as i64;
    new_clock.epoch = (new_clock.epoch as i64 + epochs_since_epoch_start) as u64;

    ctx.set_sysvar(&new_clock);
    let clock_sysvar: Clock = ctx.banks_client.get_sysvar().await.unwrap();
    println!(
        "New Time: epoch = {}, timestamp = {}",
        clock_sysvar.epoch, clock_sysvar.unix_timestamp
    );

    let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

    ctx.last_blockhash = blockhash;
}

pub async fn create_and_fund_multiple_accounts(
    program_test: &mut ProgramTest,
    number: usize,
) -> Vec<Keypair> {
    let mut keypairs = Vec::new();

    for _ in 0..number {
        keypairs.push(Keypair::new());
    }

    keypairs
        .iter()
        .for_each(|k| create_and_fund_account(&k.pubkey(), program_test));

    keypairs
}

pub async fn create_and_simulate_cortex_view_ix<T: InstructionData, U: BorshDeserialize>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    accounts_meta: Vec<AccountMeta>,
    args: T,
    payer: &Keypair,
    pre_ix: Option<solana_sdk::instruction::Instruction>,
    post_ix: Option<solana_sdk::instruction::Instruction>,
) -> std::result::Result<U, BanksClientError> {
    let ix = solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: accounts_meta,
        data: args.data(),
    };

    let payer_pubkey = payer.pubkey();

    let mut ctx = program_test_ctx.write().await;
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let mut instructions: Vec<solana_sdk::instruction::Instruction> = Vec::new();

    // Max allowed compute unit
    // https://github.com/solana-labs/solana/blob/5c2d7b6b8ae2afb443facfa5e7f608dbb41d3fdc/program-runtime/src/compute_budget_processor.rs#L19C1-L19C51
    instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));

    if let Some(pre_ix) = pre_ix {
        instructions.push(pre_ix);
    }

    instructions.push(ix);

    if let Some(post_ix) = post_ix {
        instructions.push(post_ix);
    }

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        instructions.as_slice(),
        Some(&payer_pubkey),
        &[payer],
        last_blockhash,
    );

    let result = banks_client.simulate_transaction(tx).await;

    if result.is_err() {
        return Err(result.err().unwrap());
    }

    // Extract the returned data
    let mut return_data: Vec<u8> = result
        .unwrap()
        .simulation_details
        .unwrap()
        .return_data
        .unwrap()
        .data;

    let result_expected_len = std::mem::size_of::<U>();

    // Returned data doesn't contains leading zeros, need to re-add them before deserialization
    while return_data.len() < result_expected_len {
        return_data.push(0u8);
    }

    Ok(U::try_from_slice(return_data.as_slice()).unwrap())
}

// Return the instruction size and the compute units consumed
pub async fn create_and_execute_adrena_ix<T: InstructionData, U: Signers>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    accounts_meta: Vec<AccountMeta>,
    args: T,
    payer: Option<&Pubkey>,
    signing_keypairs: &U,
    pre_ix: Option<Vec<solana_sdk::instruction::Instruction>>,
    post_ix: Option<Vec<solana_sdk::instruction::Instruction>>,
) -> std::result::Result<(usize, u64), BanksClientError> {
    // Returns main instruction size
    let ix = solana_sdk::instruction::Instruction {
        program_id: adrena::id(),
        accounts: accounts_meta,
        data: args.data(),
    };

    let ix_size = serialize(&ix).unwrap().len();

    let mut ctx = program_test_ctx.write().await;
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let mut instructions: Vec<solana_sdk::instruction::Instruction> = Vec::new();

    // Max allowed compute unit
    // https://github.com/solana-labs/solana/blob/5c2d7b6b8ae2afb443facfa5e7f608dbb41d3fdc/program-runtime/src/compute_budget_processor.rs#L19C1-L19C51
    instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));

    if let Some(pre_ix) = pre_ix {
        for ix in pre_ix {
            instructions.push(ix);
        }
    }

    instructions.push(ix.clone());

    if let Some(post_ix) = post_ix {
        for ix in post_ix {
            instructions.push(ix);
        }
    }

    let tx: Transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        instructions.as_slice(),
        payer,
        signing_keypairs,
        last_blockhash,
    );

    let ret = banks_client
        .process_transaction_with_metadata(tx)
        .await
        .unwrap();

    if ret.result.is_err() {
        return Err(ret.result.err().unwrap().into());
    }

    Ok((ix_size, ret.metadata.unwrap().compute_units_consumed))
}

pub async fn create_and_execute_spl_governance_ix<U: Signers>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    accounts_meta: Vec<AccountMeta>,
    data: Vec<u8>,
    payer: Option<&Pubkey>,
    signing_keypairs: &U,
) -> std::result::Result<(), BanksClientError> {
    let ix = solana_sdk::instruction::Instruction {
        program_id: SplGovernanceV3Adapter::id(),
        accounts: accounts_meta,
        data,
    };

    let mut ctx = program_test_ctx.write().await;
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        payer,
        signing_keypairs,
        last_blockhash,
    );

    let result = banks_client.process_transaction(tx).await;

    if result.is_err() {
        return Err(result.err().unwrap());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn set_custody_ratios(
    program_test_ctx: &RwLock<ProgramTestContext>,
    custody_admin: &Keypair,
    payer: &Keypair,
    custody_pda: &Pubkey,
    ratios: [TokenRatios; MAX_CUSTODIES],
) {
    let custody_account = get_zero_copy_account::<Custody>(program_test_ctx, *custody_pda).await;

    test_instructions::set_custody_config(
        program_test_ctx,
        custody_admin,
        payer,
        &custody_account.pool,
        custody_pda,
        SetCustodyConfigParams {
            is_stable: custody_account.is_stable != 0,
            oracle: custody_account.oracle,
            trade_oracle: custody_account.trade_oracle,
            pricing: custody_account.pricing,
            fees: custody_account.fees,
            borrow_rate: custody_account.borrow_rate,
            ratios,
        },
    )
    .await
    .unwrap();
}

#[derive(Clone, Copy)]
pub struct SetupCustodyInfo {
    pub oracle: LimitedString,
    pub trade_oracle: LimitedString,
    pub custody_pda: Pubkey,
    pub mint: Pubkey,
}

pub fn scale(amount: u64, decimals: u8) -> u64 {
    amount * 10u64.pow(decimals as u32)
}

pub fn ratio_from_percentage(percentage: f64) -> u16 {
    (Cortex::BPS_POWER as f64)
        .mul(percentage)
        .div(100_f64)
        .floor() as u16
}

pub async fn initialize_users_token_accounts(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    mints: Vec<Pubkey>,
    users: Vec<Pubkey>,
) {
    for mint in mints {
        initialize_token_accounts(program_test_ctx, mint, payer, users.as_slice())
            .await
            .unwrap();
    }
}

async fn get_zero_copy_account_info<'a>(
    program_test_ctx: &'a RwLock<ProgramTestContext>,
    pda: &'a Pubkey,
) -> std::result::Result<AccountInfo<'a>, tokio::io::Error> {
    let lamports = Box::new(1_000_000);
    let owner = Box::new(Adrena::id());

    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let account = banks_client.get_account(*pda).await.unwrap().unwrap();

    let data = account.data.clone();

    let data_box = Box::new(data);

    Ok(AccountInfo::new(
        pda,
        false,
        true,
        Box::leak(lamports),
        // Serialize `CustomOracle` struct to Vec<u8>
        Box::leak(data_box).as_mut(),
        Box::leak(owner),
        false,
        Epoch::default(),
    ))
}

async fn get_custody_account_info<'a>(
    program_test_ctx: &'a RwLock<ProgramTestContext>,
    pda: &'a Pubkey,
) -> std::result::Result<AccountInfo<'a>, tokio::io::Error> {
    get_zero_copy_account_info(program_test_ctx, pda).await
}

pub async fn get_assets_under_management_usd(
    program_test_ctx: &RwLock<ProgramTestContext>,
    pool_pda: Pubkey,
) -> std::result::Result<u128, anchor_lang::error::Error> {
    let pool_account = get_zero_copy_account::<Pool>(program_test_ctx, pool_pda).await;

    let mut account_infos: Vec<AccountInfo> = vec![];

    // Add all custodies accounts
    for custody_pda in &pool_account.custodies {
        if custody_pda.ne(&Pubkey::default()) {
            account_infos.push(
                get_custody_account_info(program_test_ctx, custody_pda)
                    .await
                    .unwrap(),
            );
        }
    }

    let current_time = get_current_unix_timestamp(program_test_ctx).await;

    let mut oracle_prices = get_account::<Oracle>(program_test_ctx, pda::get_oracle_pda().0).await;

    // Refresh timestamp
    oracle_prices.prices.iter_mut().for_each(|price| {
        if price.price > 0 {
            price.timestamp = current_time;
        }
    });

    pool_account.get_assets_under_management_usd(
        &oracle_prices,
        account_infos.as_slice(),
        current_time,
    )
}

//
// Utility functions based of bonfida-utils
// https://github.com/Bonfida/bonfida-utils/tree/9313cc372746d361c4d0c3715ea777b4a385eb30/crates/test-utils
//

pub async fn sign_send_instructions(
    program_test_ctx: &RwLock<ProgramTestContext>,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> std::result::Result<(), BanksClientError> {
    let mut ctx = program_test_ctx.write().await;

    let mut transaction = Transaction::new_with_payer(instructions, Some(&ctx.payer.pubkey()));

    let mut s = signers.to_vec();

    s.push(&ctx.payer);

    transaction.partial_sign(&s, ctx.last_blockhash);

    ctx.banks_client.process_transaction(transaction).await?;

    Ok(())
}

async fn mint_tokens_internal(
    program_test_ctx: &RwLock<ProgramTestContext>,
    mint_authority: &Keypair,
    mint_pubkey: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
) -> std::result::Result<(), BanksClientError> {
    let mint_instruction = spl_token::instruction::mint_to(
        &spl_token::ID,
        mint_pubkey,
        token_account,
        &mint_authority.pubkey(),
        &[&mint_authority.pubkey()],
        amount,
    )
    .unwrap();

    sign_send_instructions(program_test_ctx, &[mint_instruction], &[mint_authority]).await?;

    Ok(())
}

pub async fn add_mint(
    program_test: &mut ProgramTest,
    key: Option<Pubkey>,
    decimals: u8,
    mint_authority: &Pubkey,
) -> std::result::Result<(Pubkey, Mint), BanksClientError> {
    let address = key.unwrap_or_else(Pubkey::new_unique);
    let mint_info = Mint {
        mint_authority: Some(*mint_authority).into(),
        supply: u32::MAX.into(),
        decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };
    let mut data = [0; Mint::LEN];

    mint_info.pack_into_slice(&mut data);

    program_test.add_account(
        address,
        account::Account {
            lamports: u32::MAX.into(),
            data: data.into(),
            owner: spl_token::ID,
            executable: false,
            ..account::Account::default()
        },
    );

    Ok((address, mint_info))
}

pub async fn write_adrena_account<T: BorshDeserialize + BorshSerialize + Discriminator>(
    program_test_ctx: &RwLock<ProgramTestContext>,
    account: &T,
    address: &Pubkey,
    space: usize,
) {
    let mut ctx = program_test_ctx.write().await;
    let mut data = vec![];

    data.append(&mut T::discriminator().try_to_vec().unwrap());
    data.append(&mut account.try_to_vec().unwrap());

    let mut account_shared_data = AccountSharedData::new(u32::MAX.into(), space, &adrena::id());

    account_shared_data.set_data_from_slice(&data);

    ctx.set_account(address, &account_shared_data);
}

pub async fn write_oracle(program_test_ctx: &RwLock<ProgramTestContext>, oracle: Oracle) {
    let mut ctx = program_test_ctx.write().await;
    let mut data = vec![];

    data.append(&mut Oracle::discriminator().try_to_vec().unwrap());
    data.append(&mut oracle.try_to_vec().unwrap());

    let mut account_shared_data =
        AccountSharedData::new(u32::MAX.into(), Oracle::LEN, &Adrena::id());

    account_shared_data.set_data_from_slice(&data);

    let address = pda::get_oracle_pda().0;

    ctx.set_account(&address, &account_shared_data);
}

pub async fn initialize_token_accounts(
    program_test_ctx: &RwLock<ProgramTestContext>,
    mint: Pubkey,
    payer: &Keypair,
    owners: &[Pubkey],
) -> std::result::Result<Vec<Pubkey>, BanksClientError> {
    let mut instructions = Vec::with_capacity(owners.len());
    let mut account_keys = Vec::with_capacity(owners.len());

    for o in owners {
        let i = spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            o,
            &mint,
            &spl_token::ID,
        );

        account_keys.push(i.accounts[1].pubkey);
        instructions.push(i);
    }

    for c in instructions.chunks(10) {
        sign_send_instructions(program_test_ctx, c, &[payer]).await?;
    }

    Ok(account_keys)
}

pub async fn transfer_tokens(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    owner: &Keypair,
    from: &Pubkey,
    to: &Pubkey,
    amount: u64,
) -> Result<()> {
    let mut ctx = program_test_ctx.write().await;
    let banks_client = &mut ctx.banks_client;

    let last_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let transfer_ix =
        spl_token::instruction::transfer(&spl_token::id(), from, to, &owner.pubkey(), &[], amount)?;

    let transaction = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&payer.pubkey()),
        &[payer, owner],
        last_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();

    Ok(())
}

pub fn mute_logs() {
    solana_logger::setup_with(
        "solana_program_runtime::stable_log=off,\
         solana_bpf_loader=off,\
         solana_runtime=off,\
         solana_sdk=off,\
         solana_accounts_db=off,\
         solana=off,\
         agave=off",
    );
}

pub fn unmute_logs() {
    solana_logger::setup_with(
        "solana_program_runtime::stable_log=debug,\
        solana_bpf_loader=debug,\
        solana_runtime=debug,\
        solana_sdk=debug,\
        solana_accounts_db=debug,\
        solana=debug,\
        agave=debug",
    );
}

pub fn log_instruction(name: &str, category: &str, size_bytes: usize, compute_units: u64) {
    let mut log = INSTRUCTION_LOG.lock().unwrap();

    if let Some(existing) = log.iter_mut().find(|entry| entry.name == name) {
        existing.max_size_bytes = existing.max_size_bytes.max(size_bytes);
        existing.min_size_bytes = existing.min_size_bytes.min(size_bytes);
        existing.max_compute_units = existing.max_compute_units.max(compute_units);
        existing.min_compute_units = existing.min_compute_units.min(compute_units);
    } else {
        log.push(InstructionInfo::new(
            name,
            category,
            size_bytes,
            compute_units,
        ));
    }
}

// Load what's in the Oracle onchain account and convert it to a ChaosLabsBatchPrices
pub async fn get_oracle_prices_as_chaos_labs_bundle(
    program_test_ctx: &RwLock<ProgramTestContext>,
) -> ChaosLabsBatchPrices {
    let mut oracle_prices = get_account::<Oracle>(program_test_ctx, pda::get_oracle_pda().0).await;

    let current_time = get_current_unix_timestamp(program_test_ctx).await;

    oracle_prices.prices.iter_mut().for_each(|price| {
        price.timestamp = current_time;
    });

    generate_chaos_labs_batch_price(oracle_prices)
}

// Due to crates issues we cannot sign a message with a keypair
// Which means the signature and recovery_id are not supported in the tests
// To test the signature verification we need to use pre-generated signatures
// But we can't verify signatures with dynamic data
pub fn generate_chaos_labs_batch_price(oracle_prices: Oracle) -> ChaosLabsBatchPrices {
    let mut batch_prices = ChaosLabsBatchPrices {
        prices: vec![],
        signature: [0u8; 64],
        recovery_id: 0,
    };

    oracle_prices.prices.iter().for_each(|price| {
        if price.price > 0 {
            batch_prices
                .prices
                .push(adrena::state::chaos_labs_oracle::PriceData {
                    feed_id: price.chaos_labs_feed_id,
                    price: price.price,
                    timestamp: price.timestamp,
                });
        }
    });

    let message = batch_prices.build_message_hash().unwrap();

    let chaos_labs_dummy_keypair: [u8; 32] = [
        66, 177, 20, 182, 158, 64, 181, 41, 237, 161, 161, 151, 243, 243, 97, 18, 251, 100, 193,
        211, 46, 226, 229, 102, 175, 240, 154, 223, 69, 244, 20, 171,
    ];

    let sign = sign_message(&message, &chaos_labs_dummy_keypair);

    batch_prices.signature = sign.0;
    batch_prices.recovery_id = sign.1;

    batch_prices
}

// Utility function used to generate ChaosLabs batch prices compatible signatures
pub fn sign_message(message_bytes: &[u8], secret_key_bytes: &[u8; 32]) -> ([u8; 64], u8) {
    let message = libsecp256k1::Message::parse_slice(message_bytes).expect("32-byte hash");

    // Parse the secret key
    let secret = libsecp256k1::SecretKey::parse(secret_key_bytes).expect("valid secp256k1 key");

    // Sign it
    let (sig, recid) = libsecp256k1::sign(&message, &secret);

    // Format signature as [r || s] and recovery_id
    (sig.serialize(), recid.serialize())
}
