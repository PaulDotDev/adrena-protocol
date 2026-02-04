pub mod adapters;
pub mod error;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;
pub mod utils;

use {
    anchor_lang::prelude::*,
    instructions::*,
    state::cortex::{
        AmountAndFee, ExitPriceAndFee, NewPositionPricesAndFee, OpenPositionWithSwapAmountAndFees,
        ProfitAndLoss, SwapAmountAndFees,
    },
};

solana_security_txt::security_txt! {
    name: "Adrena",
    project_url: "https://github.com/AdrenaFoundation/adrena",
    contacts: "email:orex@adrena.xyz",
    policy: "",
    preferred_languages: "en",
    auditors: "offside labs, ottersec"
}

declare_id!("13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet");

#[program]
pub mod adrena {
    use super::*;

    pub fn init_one_core<'info>(
        ctx: Context<'_, '_, '_, 'info, InitOne<'info>>,
        params: InitOneParams,
    ) -> Result<()> {
        instructions::init_one_core(ctx, &params)
    }

    pub fn init_two_lm_token_metadata<'info>(
        ctx: Context<'_, '_, '_, 'info, InitTwoLmTokenMetadata<'info>>,
    ) -> Result<()> {
        instructions::init_two_lm_token_metadata(ctx)
    }

    pub fn init_three_governance<'info>(
        ctx: Context<'_, '_, '_, 'info, InitThreeGovernance<'info>>,
    ) -> Result<()> {
        instructions::init_three_governance(ctx)
    }

    pub fn init_four_vesting<'info>(
        ctx: Context<'_, '_, '_, 'info, InitFourVesting<'info>>,
    ) -> Result<()> {
        instructions::init_four_vesting(ctx)
    }

    pub fn add_vest<'info>(
        ctx: Context<'_, '_, '_, 'info, AddVest<'info>>,
        params: AddVestParams,
    ) -> Result<u8> {
        instructions::add_vest(ctx, &params)
    }

    pub fn set_vest_delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, SetVestDelegate<'info>>,
        params: SetVestDelegateParams,
    ) -> Result<()> {
        instructions::set_vest_delegate(ctx, &params)
    }

    pub fn migrate_vest_from_v1_to_v2<'info>(
        ctx: Context<'_, '_, '_, 'info, MigrateVestFromV1ToV2<'info>>,
    ) -> Result<()> {
        instructions::migrate_vest_from_v1_to_v2(ctx)
    }

    pub fn migrate_user_profile_from_v1_to_v2<'info>(
        ctx: Context<'_, '_, '_, 'info, MigrateUserProfileFromV1ToV2<'info>>,
        params: MigrateUserProfileFromV1ToV2Params,
    ) -> Result<()> {
        instructions::migrate_user_profile_from_v1_to_v2(ctx, &params)
    }

    pub fn claim_vest<'info>(ctx: Context<'_, '_, '_, 'info, ClaimVest<'info>>) -> Result<u64> {
        instructions::claim_vest(ctx)
    }

    pub fn add_pool_part_one<'info>(
        ctx: Context<'_, '_, '_, 'info, AddPoolPartOne<'info>>,
        params: AddPoolPartOneParams,
    ) -> Result<u8> {
        instructions::add_pool_part_one(ctx, &params)
    }

    pub fn add_pool_part_two<'info>(
        ctx: Context<'_, '_, '_, 'info, AddPoolPartTwo<'info>>,
        params: AddPoolPartTwoParams,
    ) -> Result<u8> {
        instructions::add_pool_part_two(ctx, &params)
    }

    pub fn remove_pool<'info>(ctx: Context<'_, '_, '_, 'info, RemovePool<'info>>) -> Result<u8> {
        instructions::remove_pool(ctx)
    }

    pub fn add_custody<'info>(
        ctx: Context<'_, '_, '_, 'info, AddCustody<'info>>,
        params: AddCustodyParams,
    ) -> Result<u8> {
        instructions::add_custody(ctx, &params)
    }

    pub fn remove_custody<'info>(
        ctx: Context<'_, '_, '_, 'info, RemoveCustody<'info>>,
        params: RemoveCustodyParams,
    ) -> Result<u8> {
        instructions::remove_custody(ctx, &params)
    }

    pub fn set_custody_config<'info>(
        ctx: Context<'_, '_, '_, 'info, SetCustodyConfig<'info>>,
        params: SetCustodyConfigParams,
    ) -> Result<u8> {
        instructions::set_custody_config(ctx, &params)
    }

    pub fn set_custody_allow_swap<'info>(
        ctx: Context<'_, '_, '_, 'info, SetCustodyAllowSwap<'info>>,
        params: SetCustodyAllowSwapParams,
    ) -> Result<()> {
        instructions::set_custody_allow_swap(ctx, &params)
    }

    pub fn set_custody_allow_trade<'info>(
        ctx: Context<'_, '_, '_, 'info, SetCustodyAllowTrade<'info>>,
        params: SetCustodyAllowTradeParams,
    ) -> Result<()> {
        instructions::set_custody_allow_trade(ctx, &params)
    }

    pub fn set_pool_allow_swap<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPoolAllowSwap<'info>>,
        params: SetPoolAllowSwapParams,
    ) -> Result<()> {
        instructions::set_pool_allow_swap(ctx, &params)
    }

    pub fn set_pool_allow_trade<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPoolAllowTrade<'info>>,
        params: SetPoolAllowTradeParams,
    ) -> Result<()> {
        instructions::set_pool_allow_trade(ctx, &params)
    }

    pub fn set_pool_aum_soft_cap_usd<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPoolAumSoftCapUsd<'info>>,
        params: SetPoolAumSoftCapUsdParams,
    ) -> Result<()> {
        instructions::set_pool_aum_soft_cap_usd(ctx, &params)
    }

    pub fn swap(ctx: Context<Swap>, params: SwapParams) -> Result<()> {
        instructions::swap(ctx, &params)
    }

    pub fn add_liquidity(ctx: Context<AddLiquidity>, params: AddLiquidityParams) -> Result<()> {
        instructions::add_liquidity(ctx, &params)
    }

    pub fn genesis_otc_out(ctx: Context<GenesisOtcOut>) -> Result<()> {
        instructions::genesis_otc_out(ctx)
    }

    pub fn genesis_otc_in(ctx: Context<GenesisOtcIn>, params: GenesisOtcInParams) -> Result<()> {
        instructions::genesis_otc_in(ctx, &params)
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        params: RemoveLiquidityParams,
    ) -> Result<()> {
        instructions::remove_liquidity(ctx, &params)
    }

    pub fn open_position_long(
        ctx: Context<OpenPositionLong>,
        params: OpenPositionLongParams,
    ) -> Result<()> {
        instructions::open_position_long(ctx, &params)
    }

    pub fn open_position_short(
        ctx: Context<OpenPositionShort>,
        params: OpenPositionShortParams,
    ) -> Result<()> {
        instructions::open_position_short(ctx, &params)
    }

    pub fn open_or_increase_position_with_swap_long(
        ctx: Context<OpenOrIncreasePositionWithSwapLong>,
        params: OpenPositionWithSwapParams,
    ) -> Result<()> {
        instructions::open_or_increase_position_with_swap_long(ctx, &params)
    }

    pub fn open_or_increase_position_with_swap_short(
        ctx: Context<OpenOrIncreasePositionWithSwapShort>,
        params: OpenPositionWithSwapParams,
    ) -> Result<()> {
        instructions::open_or_increase_position_with_swap_short(ctx, &params)
    }

    pub fn add_collateral_long(
        ctx: Context<AddCollateralLong>,
        params: AddCollateralLongParams,
    ) -> Result<()> {
        instructions::add_collateral_long(ctx, &params)
    }

    pub fn add_collateral_short(
        ctx: Context<AddCollateralShort>,
        params: AddCollateralShortParams,
    ) -> Result<()> {
        instructions::add_collateral_short(ctx, &params)
    }

    pub fn remove_collateral_long(
        ctx: Context<RemoveCollateralLong>,
        params: RemoveCollateralLongParams,
    ) -> Result<()> {
        instructions::remove_collateral_long(ctx, &params)
    }

    pub fn remove_collateral_short(
        ctx: Context<RemoveCollateralShort>,
        params: RemoveCollateralShortParams,
    ) -> Result<()> {
        instructions::remove_collateral_short(ctx, &params)
    }

    pub fn close_position_long(
        ctx: Context<ClosePositionLong>,
        params: ClosePositionLongParams,
    ) -> Result<()> {
        instructions::close_position_long(ctx, &params)
    }

    pub fn close_position_short(
        ctx: Context<ClosePositionShort>,
        params: ClosePositionShortParams,
    ) -> Result<()> {
        instructions::close_position_short(ctx, &params)
    }

    pub fn liquidate_long(ctx: Context<LiquidateLong>, params: LiquidateLongParams) -> Result<()> {
        instructions::liquidate_long(ctx, &params)
    }

    pub fn liquidate_short(
        ctx: Context<LiquidateShort>,
        params: LiquidateShortParams,
    ) -> Result<()> {
        instructions::liquidate_short(ctx, &params)
    }

    pub fn update_pool_aum(
        ctx: Context<UpdatePoolAum>,
        params: UpdatePoolAumParams,
    ) -> Result<u128> {
        instructions::update_pool_aum(ctx, &params)
    }

    pub fn get_add_liquidity_amount_and_fee(
        ctx: Context<GetAddLiquidityAmountAndFee>,
        params: GetAddLiquidityAmountAndFeeParams,
    ) -> Result<AmountAndFee> {
        instructions::get_add_liquidity_amount_and_fee(ctx, &params)
    }

    pub fn get_remove_liquidity_amount_and_fee(
        ctx: Context<GetRemoveLiquidityAmountAndFee>,
        params: GetRemoveLiquidityAmountAndFeeParams,
    ) -> Result<AmountAndFee> {
        instructions::get_remove_liquidity_amount_and_fee(ctx, &params)
    }

    pub fn get_entry_price_and_fee(
        ctx: Context<GetEntryPriceAndFee>,
        params: GetEntryPriceAndFeeParams,
    ) -> Result<NewPositionPricesAndFee> {
        instructions::get_entry_price_and_fee(ctx, &params)
    }

    pub fn disable_tokens_freeze_capabilities(
        ctx: Context<DisableTokensFreezeCapabilities>,
    ) -> Result<()> {
        instructions::disable_tokens_freeze_capabilities(ctx)
    }

    pub fn genesis_stake_patch(ctx: Context<GenesisStakePatch>) -> Result<()> {
        instructions::genesis_stake_patch(ctx)
    }

    pub fn get_open_position_with_swap_amount_and_fees(
        ctx: Context<GetOpenPositionWithSwapAmountAndFees>,
        params: GetOpenPositionWithSwapAmountAndFeesParams,
    ) -> Result<OpenPositionWithSwapAmountAndFees> {
        instructions::get_open_position_with_swap_amount_and_fees(ctx, &params)
    }

    pub fn get_exit_price_and_fee(
        ctx: Context<GetExitPriceAndFee>,
        params: GetExitPriceAndFeeParams,
    ) -> Result<ExitPriceAndFee> {
        instructions::get_exit_price_and_fee(ctx, &params)
    }

    pub fn get_pnl(ctx: Context<GetPnl>, params: GetPnlParams) -> Result<ProfitAndLoss> {
        instructions::get_pnl(ctx, &params)
    }

    pub fn get_liquidation_price(
        ctx: Context<GetLiquidationPrice>,
        params: GetLiquidationPriceParams,
    ) -> Result<u64> {
        instructions::get_liquidation_price(ctx, &params)
    }

    pub fn get_liquidation_state(
        ctx: Context<GetLiquidationState>,
        params: GetLiquidationStateParams,
    ) -> Result<u8> {
        instructions::get_liquidation_state(ctx, &params)
    }

    pub fn get_swap_amount_and_fees(
        ctx: Context<GetSwapAmountAndFees>,
        params: GetSwapAmountAndFeesParams,
    ) -> Result<SwapAmountAndFees> {
        instructions::get_swap_amount_and_fees(ctx, &params)
    }

    pub fn get_assets_under_management(
        ctx: Context<GetAssetsUnderManagement>,
        params: GetAssetsUnderManagementParams,
    ) -> Result<u128> {
        instructions::get_assets_under_management(ctx, &params)
    }

    pub fn init_user_staking(ctx: Context<InitUserStaking>) -> Result<()> {
        instructions::init_user_staking(ctx)
    }

    pub fn init_user_profile(
        ctx: Context<InitUserProfile>,
        params: InitUserProfileParams,
    ) -> Result<()> {
        instructions::init_user_profile(ctx, &params)
    }

    pub fn edit_user_profile(
        ctx: Context<EditUserProfile>,
        params: EditUserProfileParams,
    ) -> Result<()> {
        instructions::edit_user_profile(ctx, &params)
    }

    pub fn edit_user_profile_nickname(
        ctx: Context<EditUserProfileNickname>,
        params: EditUserProfileNicknameParams,
    ) -> Result<()> {
        instructions::edit_user_profile_nickname(ctx, &params)
    }

    pub fn delete_user_profile(ctx: Context<DeleteUserProfile>) -> Result<()> {
        instructions::delete_user_profile(ctx)
    }

    pub fn init_staking_one<'info>(
        ctx: Context<'_, '_, '_, 'info, InitStakingOne<'info>>,
        params: InitStakingOneParams,
    ) -> Result<u8> {
        instructions::init_staking_one(ctx, &params)
    }

    pub fn init_staking_two<'info>(
        ctx: Context<'_, '_, '_, 'info, InitStakingTwo<'info>>,
    ) -> Result<u8> {
        instructions::init_staking_two(ctx)
    }

    pub fn init_staking_three<'info>(
        ctx: Context<'_, '_, '_, 'info, InitStakingThree<'info>>,
    ) -> Result<u8> {
        instructions::init_staking_three(ctx)
    }

    pub fn init_staking_four<'info>(
        ctx: Context<'_, '_, '_, 'info, InitStakingFour<'info>>,
    ) -> Result<u8> {
        instructions::init_staking_four(ctx)
    }

    pub fn add_liquid_stake(
        ctx: Context<AddLiquidStake>,
        params: AddLiquidStakeParams,
    ) -> Result<()> {
        instructions::add_liquid_stake(ctx, &params)
    }

    pub fn add_locked_stake(
        ctx: Context<AddLockedStake>,
        params: AddLockedStakeParams,
    ) -> Result<()> {
        instructions::add_locked_stake(ctx, &params)
    }

    pub fn upgrade_locked_stake(
        ctx: Context<UpgradeLockedStake>,
        params: UpgradeLockedStakeParams,
    ) -> Result<()> {
        instructions::upgrade_locked_stake(ctx, &params)
    }

    pub fn remove_liquid_stake(
        ctx: Context<RemoveLiquidStake>,
        params: RemoveLiquidStakeParams,
    ) -> Result<()> {
        instructions::remove_liquid_stake(ctx, &params)
    }

    pub fn remove_locked_stake(
        ctx: Context<RemoveLockedStake>,
        params: RemoveLockedStakeParams,
    ) -> Result<()> {
        instructions::remove_locked_stake(ctx, &params)
    }

    pub fn claim_stakes(ctx: Context<ClaimStakes>, params: ClaimStakesParams) -> Result<()> {
        instructions::claim_stakes(ctx, &params)
    }

    pub fn finalize_genesis_lock_campaign<'info>(
        ctx: Context<'_, '_, '_, 'info, FinalizeGenesisLockCampaign<'info>>,
    ) -> Result<()> {
        instructions::finalize_genesis_lock_campaign(ctx)
    }

    pub fn set_pool_liquidity_state(
        ctx: Context<SetPoolLiquidityState>,
        params: SetPoolLiquidityStateParams,
    ) -> Result<()> {
        instructions::set_pool_liquidity_state(ctx, &params)
    }

    pub fn finalize_locked_stake<'info>(
        ctx: Context<'_, '_, '_, 'info, FinalizeLockedStake<'info>>,
        params: FinalizeLockedStakeParams,
    ) -> Result<()> {
        instructions::finalize_locked_stake(ctx, &params)
    }

    pub fn resolve_staking_round(ctx: Context<ResolveStakingRound>) -> Result<()> {
        instructions::resolve_staking_round(ctx)
    }

    pub fn get_lp_token_price(
        ctx: Context<GetLpTokenPrice>,
        params: GetLpTokenPriceParams,
    ) -> Result<u64> {
        instructions::get_lp_token_price(ctx, &params)
    }

    pub fn get_pool_info_snapshot(
        ctx: Context<GetPoolInfoSnapshot>,
        params: GetPoolInfoSnapshotParams,
    ) -> Result<PoolInfoSnapshot> {
        instructions::get_pool_info_snapshot(ctx, &params)
    }

    pub fn mint_lm_tokens_from_bucket<'info>(
        ctx: Context<'_, '_, '_, 'info, MintLmTokensFromBucket<'info>>,
        params: MintLmTokensFromBucketParams,
    ) -> Result<u8> {
        instructions::mint_lm_tokens_from_bucket(ctx, &params)
    }

    pub fn increase_position_long(
        ctx: Context<IncreasePositionLong>,
        params: IncreasePositionLongParams,
    ) -> Result<()> {
        instructions::increase_position_long(ctx, &params)
    }

    pub fn patch_custody_locked_amount(ctx: Context<PatchCustodyLockedAmount>) -> Result<()> {
        instructions::patch_custody_locked_amount(ctx)
    }

    pub fn increase_position_short(
        ctx: Context<IncreasePositionShort>,
        params: IncreasePositionShortParams,
    ) -> Result<()> {
        instructions::increase_position_short(ctx, &params)
    }

    pub fn set_staking_lm_emission_potentiometers<'info>(
        ctx: Context<'_, '_, '_, 'info, SetStakingLmEmissionPotentiometers<'info>>,
        params: SetStakingLmEmissionPotentiometersParams,
    ) -> Result<()> {
        instructions::set_staking_lm_emission_potentiometers(ctx, &params)
    }

    pub fn set_admin<'info>(
        ctx: Context<'_, '_, '_, 'info, SetAdmin<'info>>,
        params: SetAdminParams,
    ) -> Result<()> {
        instructions::set_admin(ctx, &params)
    }

    pub fn set_protocol_fee_recipient(ctx: Context<SetProtocolFeeRecipient>) -> Result<()> {
        instructions::set_protocol_fee_recipient(ctx)
    }

    pub fn set_custody_max_cumulative_short_position_size_usd<'info>(
        ctx: Context<'_, '_, '_, 'info, SetCustodyMaxCumulativeShortPositionSizeUsd<'info>>,
        params: SetCustodyMaxCumulativeShortPositionSizeUsdParams,
    ) -> Result<()> {
        instructions::set_custody_max_cumulative_short_position_size_usd(ctx, &params)
    }

    pub fn set_take_profit_long<'info>(
        ctx: Context<'_, '_, '_, 'info, SetTakeProfitLong<'info>>,
        params: SetTakeProfitLongParams,
    ) -> Result<()> {
        instructions::set_take_profit_long(ctx, &params)
    }

    pub fn set_stop_loss_long<'info>(
        ctx: Context<'_, '_, '_, 'info, SetStopLossLong<'info>>,
        params: SetStopLossLongParams,
    ) -> Result<()> {
        instructions::set_stop_loss_long(ctx, &params)
    }

    pub fn set_take_profit_short<'info>(
        ctx: Context<'_, '_, '_, 'info, SetTakeProfitShort<'info>>,
        params: SetTakeProfitShortParams,
    ) -> Result<()> {
        instructions::set_take_profit_short(ctx, &params)
    }

    pub fn set_stop_loss_short<'info>(
        ctx: Context<'_, '_, '_, 'info, SetStopLossShort<'info>>,
        params: SetStopLossShortParams,
    ) -> Result<()> {
        instructions::set_stop_loss_short(ctx, &params)
    }

    pub fn cancel_take_profit<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelTakeProfit<'info>>,
    ) -> Result<()> {
        instructions::cancel_take_profit(ctx)
    }

    pub fn cancel_stop_loss<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelStopLoss<'info>>,
    ) -> Result<()> {
        instructions::cancel_stop_loss(ctx)
    }

    pub fn patch_staking_round(ctx: Context<PatchStakingRound>) -> Result<()> {
        instructions::patch_staking_round(ctx)
    }

    pub fn set_pool_whitelisted_swapper<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPoolWhitelistedSwapper<'info>>,
    ) -> Result<()> {
        instructions::set_pool_whitelisted_swapper(ctx)
    }

    pub fn init_limit_order_book<'info>(
        ctx: Context<'_, '_, '_, 'info, InitLimitOrderBook<'info>>,
    ) -> Result<()> {
        instructions::init_limit_order_book(ctx)
    }

    pub fn add_limit_order<'info>(
        ctx: Context<'_, '_, '_, 'info, AddLimitOrder<'info>>,
        params: AddLimitOrderParams,
    ) -> Result<u64> {
        instructions::add_limit_order(ctx, &params)
    }

    pub fn cancel_limit_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelLimitOrder<'info>>,
        params: CancelLimitOrderParams,
    ) -> Result<()> {
        instructions::cancel_limit_order(ctx, &params)
    }

    pub fn execute_limit_order_long<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteLimitOrderLong<'info>>,
        params: ExecuteLimitOrderLongParams,
    ) -> Result<()> {
        instructions::execute_limit_order_long(ctx, &params)
    }

    pub fn execute_limit_order_short<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteLimitOrderShort<'info>>,
        params: ExecuteLimitOrderShortParams,
    ) -> Result<()> {
        instructions::execute_limit_order_short(ctx, &params)
    }

    pub fn distribute_fees(
        ctx: Context<DistributeFees>,
        params: DistributeFeesParams,
    ) -> Result<()> {
        instructions::distribute_fees(ctx, &params)
    }

    pub fn claim_referral_fee(ctx: Context<ClaimReferralFee>) -> Result<()> {
        instructions::claim_referral_fee(ctx)
    }

    pub fn init_referrer_reward_token_vault(
        ctx: Context<InitReferrerRewardTokenVault>,
    ) -> Result<()> {
        instructions::init_referrer_reward_token_vault(ctx)
    }

    pub fn grant_or_remove_achievement(
        ctx: Context<GrantOrRemoveAchievement>,
        params: GrantOrRemoveAchievementParams,
    ) -> Result<()> {
        instructions::grant_or_remove_achievement(ctx, params)
    }

    pub fn init_oracle(ctx: Context<InitOracle>, params: InitOracleParams) -> Result<()> {
        instructions::init_oracle(ctx, &params)
    }

    pub fn patch_custodies_oracles(ctx: Context<PatchCustodiesOracles>) -> Result<()> {
        instructions::patch_custodies_oracles(ctx)
    }

    pub fn resolve_position_borrow_fees(
        ctx: Context<ResolvePositionBorrowFees>,
        params: ResolvePositionBorrowFeesParams,
    ) -> Result<()> {
        instructions::resolve_position_borrow_fees(ctx, &params)
    }

    pub fn sync_user_voting_power(ctx: Context<SyncUserVotingPower>) -> Result<()> {
        instructions::sync_user_voting_power(ctx)
    }

    pub fn mint_staked_lm_tokens_from_bucket<'info>(
        ctx: Context<'_, '_, '_, 'info, MintStakedLmTokensFromBucket<'info>>,
        params: MintStakedLmTokensFromBucketParams,
    ) -> Result<()> {
        instructions::mint_staked_lm_tokens_from_bucket(ctx, &params)
    }
}
