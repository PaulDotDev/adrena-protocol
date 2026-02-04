use {
    super::pda,
    crate::{
        test_instructions::{
            claim_stakes, close_position_long_permissionless, close_position_short_permissionless,
            finalize_genesis_lock_campaign, finalize_locked_stake, resolve_staking_round,
        },
        utils::{self},
    },
    adrena::{
        instructions::{ClaimStakesParams, ClosePositionLongParams, ClosePositionShortParams},
        state::{
            chaos_labs_oracle::ChaosLabsBatchPrices, cortex::Cortex, position::Position,
            user_staking::UserStaking,
        },
    },
    anchor_lang::prelude::Pubkey,
    num::Zero,
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::keypair::Keypair,
    tokio::sync::RwLock,
};

pub async fn execute_take_profit_long_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    position_pda: &Pubkey,
    payer: &Keypair,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<bool, BanksClientError> {
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    match close_position_long_permissionless(
        program_test_ctx,
        payer,
        &position_account.owner,
        position_pda,
        ClosePositionLongParams {
            price: Some(position_account.take_profit_limit_price),
            oracle_prices: oracle_prices.clone(),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error closing position long: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_take_profit_short_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    position_pda: &Pubkey,
    payer: &Keypair,
) -> std::result::Result<bool, BanksClientError> {
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    match close_position_short_permissionless(
        program_test_ctx,
        payer,
        &position_account.owner,
        position_pda,
        ClosePositionShortParams {
            price: Some(position_account.take_profit_limit_price),
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error closing position short: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_stop_loss_long_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    position_pda: &Pubkey,
    payer: &Keypair,
    oracle_prices: Option<ChaosLabsBatchPrices>,
) -> std::result::Result<bool, BanksClientError> {
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    match close_position_long_permissionless(
        program_test_ctx,
        payer,
        &position_account.owner,
        position_pda,
        ClosePositionLongParams {
            price: if position_account.stop_loss_close_position_price.is_zero() {
                None
            } else {
                Some(position_account.stop_loss_close_position_price)
            },
            oracle_prices: oracle_prices.clone(),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error closing position long: {:?}", e);
            Ok(false)
        }
    }
}

// return true if executed, false if not
pub async fn execute_stop_loss_short_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    position_pda: &Pubkey,
    payer: &Keypair,
) -> std::result::Result<bool, BanksClientError> {
    let position_account =
        utils::get_zero_copy_account::<Position>(program_test_ctx, *position_pda).await;

    match close_position_short_permissionless(
        program_test_ctx,
        payer,
        &position_account.owner,
        position_pda,
        ClosePositionShortParams {
            price: if position_account.stop_loss_close_position_price.is_zero() {
                None
            } else {
                Some(position_account.stop_loss_close_position_price)
            },
            oracle_prices: Some(
                utils::get_oracle_prices_as_chaos_labs_bundle(program_test_ctx).await,
            ),
            percentage: Cortex::BPS_POWER as u64 * 100, // 100% of the position
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error closing position short: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_resolve_staking_round_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    staked_token_mint: &Pubkey,
) -> std::result::Result<bool, BanksClientError> {
    match resolve_staking_round(program_test_ctx, payer, payer, staked_token_mint).await {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error resolving staking round: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_finalize_genesis_lock_campaign_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    pool_pda: &Pubkey,
) -> std::result::Result<bool, BanksClientError> {
    match finalize_genesis_lock_campaign(program_test_ctx, payer, pool_pda).await {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error finalizing genesis lock campaign: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_claim_stakes_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    owner: &Pubkey,
    pool_pda: &Pubkey,
    staked_token_mint: &Pubkey,
) -> std::result::Result<bool, BanksClientError> {
    match claim_stakes(
        program_test_ctx,
        payer,
        payer,
        owner,
        pool_pda,
        staked_token_mint,
        &ClaimStakesParams {
            locked_stake_indexes: None,
        },
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error claiming stakes: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn execute_finalize_locked_stake_automation(
    program_test_ctx: &RwLock<ProgramTestContext>,
    payer: &Keypair,
    owner: &Pubkey,
    governance_realm_pda: &Pubkey,
    staked_token_mint: &Pubkey,
    locked_stake_index: usize,
) -> std::result::Result<bool, BanksClientError> {
    let staking_pda = pda::get_staking_pda(staked_token_mint).0;
    let user_staking_pda = pda::get_user_staking_pda(owner, &staking_pda).0;
    let user_staking_account =
        utils::get_zero_copy_account::<UserStaking>(program_test_ctx, user_staking_pda).await;

    let is_early_exit = user_staking_account
        .locked_stakes
        .get(locked_stake_index)
        .map_or(false, |ls| ls.is_early_exit());

    match finalize_locked_stake(
        program_test_ctx,
        payer,
        owner,
        payer,
        staked_token_mint,
        governance_realm_pda,
        locked_stake_index,
        is_early_exit,
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Error finalizing locked stake: {:?}", e);
            Ok(false)
        }
    }
}
