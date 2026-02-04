use anchor_lang::prelude::*;

#[error_code]
pub enum AdrenaError {
    #[msg("Overflow in arithmetic operation")] // 6000
    MathOverflow,
    #[msg("Unsupported price oracle")]
    UnsupportedOracle,
    #[msg("Invalid oracle account")]
    InvalidOracleAccount,
    #[msg("Invalid oracle state")]
    InvalidOracleState,
    #[msg("Stale oracle price")]
    StaleOraclePrice,
    #[msg("Invalid oracle price")]
    InvalidOraclePrice,
    #[msg("Instruction is not allowed in production")]
    InvalidEnvironment,
    #[msg("Invalid pool liquidity state")]
    InvalidPoolLiquidityState,
    #[msg("Invalid cortex state")]
    InvalidCortexState,
    #[msg("Invalid staking state")]
    InvalidStakingState,
    #[msg("Invalid pool state")]
    InvalidPoolState,
    #[msg("Invalid vest state")]
    InvalidVestState,
    #[msg("Invalid stake state")]
    InvalidStakeState,
    #[msg("Invalid custody")] // 6010
    InvalidCustody,
    #[msg("Invalid custody account")]
    InvalidCustodyAccount,
    #[msg("Invalid custody state")]
    InvalidCustodyState,
    #[msg("Invalid collateral custody")]
    InvalidCollateralCustody,
    #[msg("Invalid position state")]
    InvalidPositionState,
    #[msg("The position is not in liquidation range")]
    PositionNotInLiquidationRange,
    #[msg("Invalid staking round state")]
    InvalidStakingRoundState,
    #[msg("Invalid adrena config")]
    InvalidAdrenaConfig,
    #[msg("Invalid pool config")]
    InvalidPoolConfig,
    #[msg("Invalid custody config")]
    InvalidCustodyConfig,
    #[msg("Insufficient token amount returned")]
    InsufficientAmountReturned,
    #[msg("Price slippage limit exceeded")]
    MaxPriceSlippage,
    #[msg("Position leverage limit exceeded")]
    MaxLeverage,
    #[msg("Position leverage under minimum")]
    MinLeverage,
    #[msg("Custody amount limit exceeded")]
    CustodyAmountLimit,
    #[msg("Position amount limit exceeded")]
    PositionAmountLimit,
    #[msg("Token ratio out of range")]
    TokenRatioOutOfRange,
    #[msg("Token is not supported")]
    UnsupportedToken,
    #[msg("Instruction is not allowed at this time")]
    InstructionNotAllowed,
    #[msg("Token utilization limit exceeded")]
    MaxUtilization,
    #[msg("Max registered resolved staking round reached")]
    MaxRegisteredResolvedStakingRoundReached,
    #[msg("Governance program do not match Cortex's one")]
    InvalidGovernanceProgram,
    #[msg("Governance realm do not match Cortex's one")]
    InvalidGovernanceRealm,
    #[msg("Vesting unlock time is too close or passed")]
    InvalidVestingUnlockTime,
    #[msg("Invalid staking locking time")]
    InvalidStakingLockingTime,
    #[msg("The user stake account specified could not be found")]
    UserStakeNotFound,
    #[msg("Invalid account data")]
    InvalidAccountData,
    #[msg("Stake is not resolved")]
    UnresolvedStake,
    #[msg("Reached bucket mint limit")]
    BucketMintLimit,
    #[msg("Genesis ALP add liquidity limit reached")]
    GenesisAlpLimitReached,
    #[msg("Permissionless oracle update must be preceded by Ed25519 signature verification instruction")]
    PermissionlessOracleMissingSignature,
    #[msg("Ed25519 signature verification data does not match expected format")]
    PermissionlessOracleMalformedEd25519Data,
    #[msg("Ed25519 signature was not signed by the oracle authority")]
    PermissionlessOracleSignerMismatch,
    #[msg("Signed message does not match instruction params")]
    PermissionlessOracleMessageMismatch,
    #[msg("Cannot find custody stable locked amount")]
    CustodyStableLockedAmountNotFound,
    #[msg("Cannot find custody")]
    CustodyNotFound,
    #[msg("The bucket does not contain enough token for reserving this allocation")]
    InsufficientBucketReserve,
    #[msg("User nickname exceed 24 characters")]
    UserNicknameTooLong,
    #[msg("User nickname is less than 3 characters")]
    UserNicknameTooShort,
    #[msg("Invalid genesis lock state")]
    InvalidGenesisLockState,
    #[msg("The campaign is fully subscribed")]
    GenesisLockCampaignFullySubscribed,
    #[msg("The pool is fully subscribed")]
    PoolAumSoftCapUsdReached,
    #[msg("The number of registered pool reached max amount")]
    MaxRegisteredPool,
    #[msg("The number of registered custody reached max amount")]
    MaxRegisteredCustodies,
    #[msg("The short limit for this asset has been reached")]
    MaxCumulativeShortPositionSizeLimit,
    #[msg("The max number of LockedStaking has been reached")]
    LockedStakeArrayFull,
    #[msg("Requested index is out of bounds")]
    IndexOutOfBounds,
    #[msg("The instruction must be call with a specific account as caller")]
    InvalidCaller,
    #[msg("Invalid bucket name")]
    InvalidBucketName,
    #[msg("(deprecated)The provided Sablier thread does not have the expected ID")]
    InvalidThreadId,
    #[msg("The exponent used for pyth price lead to high precision loss")]
    PythPriceExponentTooLargeIncurringPrecisionLoss,
    #[msg("The close position price is mandatory")]
    MissingClosePositionPrice,
    #[msg("Invalid vote multiplier")]
    InvalidVoteMultiplier,
    #[msg("A position cannot be close right after open, a slight delay is enforced")]
    PositionTooYoung,
    #[msg("The minimum amount of collateral posted to open a position is not met")]
    InsufficientCollateral,
    #[msg("The provided lock duration isn't valid")]
    InvalidLockDuration,
    #[msg("The stake isn't established yet")]
    StakeNotEstablished,
    #[msg("The position is already pending cleanup and close")]
    PositionAlreadyClosed,
    #[msg("Invalid limit order state")]
    InvalidLimitOrderState,
    #[msg("Wallpaper or Profile Picture or Title is invalid")]
    InvalidWallpaperOrProfilePictureOrTitle,
    #[msg("Invalid version")]
    InvalidVersion,
    #[msg("Invalid vest version")]
    InvalidVestVersion,
    #[msg("Missing or invalid referrer account")]
    MissingOrInvalidReferrerAccount,
    #[msg("The requested wallpaper has not been unlocked by this user")]
    WallpaperNotUnlocked,
    #[msg("The requested profile picture has not been unlocked by this user")]
    ProfilePictureNotUnlocked,
    #[msg("The requested title has not been unlocked by this user")]
    TitleNotUnlocked,
    #[msg("Invalid achievement ID")]
    InvalidAchievement,
    #[msg("User nickname expected format: Monster followed by digits")]
    UserNicknameInvalidFormat,
    #[msg("Continent or Team is invalid")]
    InvalidContinentOrTeam,
    #[msg("The team can not be changed after being already set")]
    TeamImmutable,
    #[msg("Invalid signer")]
    InvalidSigner,
    #[msg("Missing at least one oracle price")]
    MissingOraclePrice,
    #[msg("Invalid oracle signature")]
    InvalidOracleSignature,
    #[msg("Custody amount is below minimum required")]
    CustodyBelowMinimum,
}
