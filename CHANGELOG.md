# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.3.7 [Unreleased]

### Updated

- Apply collateral PnL to AUM usd

## 1.3.6 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/HgAqHwuwbaDSfexFNxpq3v661htzMaZGJnVfrC4n3b3T)

### Added

- Calculate collateral_usd total for each custody in long

## 1.3.5 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/4QQtiD41mcgbV4HvcCKRG7in6NBSargqy7EVBTJorfNj)

### Fixed

- Removed confidence on USDC complementary

## 1.3.4 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/7TWTzML5DG9YDxusUkwHEnAkKXRh9AnzaCgz3udMwkCq)

### Updated

- Removed the confidence for whitelisted swaps to ease rebalancing

## 1.3.3 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/JBVMPBbXJGjw9ZrQuFDNBGcKgjoNSxJ9PrE4cDxmAd9u)

### Fixed

- Removed confidence on USDC

## 1.3.2 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/9KH9C2euQxF1Z7duSD9HeYhQpv7NKtPLZvLFevbrnAGr)

### Added

- New ix MintAndStake
- New ix ResolvePositionBorrow
- New ix SyncUserVotingPower

### Updated

- Disallow internal swap
- Remove dynamic fee slope for swaps and add, remove liquidity
- New percentage attribute in close positions to handle partial close

## 1.3.1[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/7ZmofUfzoxncYNHxgpRhowdgs7crYWNXCQioGQnkuzkE)

### Updated

- Store ALP price in the pool account

## 1.3.0[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/F71Rrdwa1Vzzy4nT2uE3av8EPGB9SnYuq5pWxGjdd34d)

### Updated

- Migrate from Pyth push model oracle to Chaos Labs Edge offchain API signature based oracle

## 1.2.4[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/56h2i9jeTe3Eku4ZCd7kV3fuo8KP4ZtWLNL33ZQgRHq7)

### Added

- Add new Achievements + Pfp + Titles + Wallpapers

### Updated

- Init user profile is now permissionless
- Migrate user profile is now permissionless

## 1.2.3[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/8ovFCNYDpUM8Ri4poEXtLkaLyznCTicotfCTRGBTJSGE)

### Fixed

- Add liquidity calculations

## 1.2.2[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/GTxrFnsKBvZTki2qUcMziyjoBxqfowPEPW28GTNP7NKP)
## 1.2.1[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/6XYPAxr8RiyCEQVSSjn7ZsU1XmyNVJWEWbkCDpiAH2YE)
## 1.2.0[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/HjTAyRUg6w8jHbFUBordBUTBio2E4k8pYyDRqd5hHv6z)

### Added

- Achievement system
- Migration to fully liquid ALP system

## 1.1.11[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/GcfPtMX5FyMrg2bs1BLKXNcNNFVtnf7JwtTogxuRmBXV)

### Fixed

- Increase position min $10 collateral + increase price calculation precision

## 1.1.10[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/C1w8d4VKU3tbFTANkCiwPpZ5mB8h6Q7hPZvupY3LpzQJ)

### Fixed

- Check on escrowed collateral for limit order book

## 1.1.9[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/3zyp6r6ziMJLvJWQxWFZUHA4daADvS4JLiUJRWY6hag6)

### Added

- Limit Order
- Profile v2

## 1.1.8[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/zHykUjnzcDFkCNtc3HX6MDWKPKAaFr1VRRm3MhfFtsY)

### Fixed

- APR 0 #211

### Audit Fixes

- 17 - External Fees Not Considered in AUM Check #213
- 18 - Liquidity Operations Moving Towards Target Ratio May be Rejected #212

### Added

- Calculate Size and CU #214
- Computing file + improvements #216
- Vest delegation #215

## 1.1.7[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/DWGtjZNWFPuwkY1AXViG7YdN2TGi7psDmQaWCgUDPm2i)

### Fixed

- Leverage checks to allow user to add collateral as long as below MAX leverage #191
- Fee distribution #208

### Audit fixes

- 14 - Add Liquidity LP Fees Distribution is Inaccurate #192
- 15 - Remove Liquidity LP Fees Distribution is Inaccurate #193
- 16 - Borrow Rate Not Updated After Fee Distribution #194
- 19 - AUM Profit Deduction Calculation Error #206
- 21 / 23 - Offside fixes #197
- A06 - close_position_short Not Update collateral_custody.collected_fees.borrow_usd #199
- A07 - OpenOrIncreasePositionWithSwapShort Instruction Missing protocol_fee_recipient Check #200
- A08 - Suggested Add Price Validation for SL Parameters #201
- A10 - Incorrect Validation in remove_custody  #202
- A09 - Incorrect Validation in remove_pool #203
- A11 - Incorrect Unique Custody Check in Pool::validate  #204
- A12 - Recommend to Check if the Locked State Has Ended When Finalizing With early_exit #205

### Added

- Add referrer for referral links #207

### Enabled

- Position Increase IXs

## 1.1.6[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/99bni5Vxb9NdR2XtYqigXuWqsXqzTXorx6ae8gG2pnkU)

### Fixed

- Position price calculation in increase position

### Enabled

- Position Increase IXs

## 1.1.5[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/C5ngg58ffxfKcNMoYrTwjdX16u5S92rW7FmP5g7Hjt54)

### Disabled

- Position Increase IXs in order to fix a bug report from Offside

## 1.1.4[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/CGTxjX5roVhvu8Cb6raYpe6PCa7y5rC9weZ1aebgjgwN)

### Fixed

- Fix critical issue from Offside related to governance attack
- Add missing unrealized_interest amount in update_accounting_after_remove_position_long

### Updated

- Log total USDC and ADX during resolve_staking_round (for @adrena-kino ) Please check that's what's needed 100%
- Use saturating add for liquidation count (short)

## 1.1.3[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/AD3yHuCi1Pb9eL23zV54B5xPW6shnT7s1nGg6qfMVATm)

### Updated

- Fix ratio check in internal swap (pay fees)
- Add pool info snapshot volume

## 1.1.2[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/Fv6aQvd3wekpvVY53rveFPbMLXVse9bBZAaXj4SWgf1R)

### Updated

- Makes swap permissioned

## 1.1.1[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/EETLmmbdqt1BZx9LnWChJjqR3yqLffFBKUYEsJjLonE1)

### Updated

- claim_stake for fixing compute unit issues

### Audit #2 fixes

- [Offside] A02 - Add check to prevent long on stable custody #175
- [Offside] 10 - Short Positions Should Also Use High Collateral Price for Accounting Update #173
- [Offside] 09 - should use high price to calculate collateral withdrawal #172
- [Offside] 08 - Increase Position with Swap Checks Incorrect Custody Flag #171
- [Offside] 04 - position increase instructions can reset liquidation fee to zero #169
- [Offside] multiple - Fix/accounting #178
- [Offside] 01 - Insufficient Position Price Precision #166
- [Offside] 06 - Enforce slippage for permissionless short position close #170

## 1.1.0[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/4KEsQdyAeF5kvKVsN6eh5waqJGH8ktgizAJeBGekfW7p)

### Removed

- Sablier (onchain automation setup) from all instructions, now handled with MrSablier (gRPC client)

### Added

- Unique ID for position
- Unique ID for locked stake per user
- Events for staking related actions

## 1.0.12[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/FQMjfWgzuYyeDThT2rT4pBF87UkpMURYU8ymheohjzVM)

### Fixed

- Fix issue with liquid staking overlap

### Added

- Add lp circulating supply in get_pool_info_snapshot view
- Add staking_type in user staking accounts
- Returns an error from get_liquidation_state if the position is in closing state

## 1.0.11[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/4cLcJPWj7f9WwRfAhqoXLj4dnBBHTsCy4KJc17oUA1hY)

### Fixed

- Issue regarding staking round rewards where too many rewards were claimed. This is due to Upgrade staking round, as the amount was upgraded but a claim wasn't done for previous rounds.

### Added

- Patch IX related to the fix above

## 1.0.10[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/6isHBoQ3gswx2dGf3sq6EfAuBB6cTKnMjHiqbQWMhKTi)

### Changed

- close_position_long/short now only close the `Position` account if the caller is the owner, if it's called permissionlessly in the context
  of a SL/TP it won't close it right away in order to do the SL/TP cleanup flow.

### Fixed

- Borrow fees are not settled correctly during a IncreasePosition IX (they were wiped back to 0)

### Added

- cleanup_position_stop_loss and cleanup_position_take_profits

## 1.0.9[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/BErXHhtxqFVhv2xZ9icrFC6oLbcLye8nCZ3zH2wWYHi4)

### Fixed

- Remove thread delete x2 from thread instructions to fix the size issue (bug)

## 1.0.8[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/3bv3CzTr3YKBycW4ypL4R4QgLDdsZCNqy5HbRnUAnyns)

### Neutralized

- GenesisStakePatch IX
- PatchCustodyLockedAmount

### Added

- set SL/TP thread_create and update now provide the necessary instruction to cleanup SL and TP thread when they trigger
  (even if they don't exist, thanks to a change in Sablier thread_delete)

### Improved

- Optimize rent paid when creating Sablier automation thread - no more double rent spending, provide for 25 ix executions + 2k prio fee per ix (returned if unused)

### Fixed

- Update the Locked Stake has_ended condition to use the end_date, as the upgrade_stake can mess with the stake_time + duration calculation
- Set stop_loss_long/short now update the trigger price (bug)

## 1.0.7[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/HzesB8auZTdXysujSL2KyGGiVoreQkxPviuHNzX7YrWY)

### Added

- Upgrade Stake IX - User can now upgrade a locked stake in quality (duration) or quantity (amount)

## 1.0.6[Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/74J5aqSwg6SGET6Y8CJWwNZPxmDm8vA6rQYWoBChDrVw)

### Tests

- Max initial leverage to 120x and max leverage to 300x (in tests, will update prod settings once released through additional governance proposals)

### Fixed

- All operations on Positions are now using the mid price as the trading price, instead of high/low variations depending of the side

## 1.0.5 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/CumV91nZSrB6HQd86bdKGPrPVzzhsCcVuhmj56sDqzYy)

### Added

- new instruction to doctor data related to LockedAmount in custody.

### Fixed

- Increase_position_long/short: add the actual increase amount and not the whole position to the locked amount.

## 1.0.4 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/EoxDw4j8eTjtR4NCdZ2K3YDiT1repuo8g4RnNV9r6awY)

### Fixed

- new instruction to fix an issue for genesis liquidity provider that are not able to collect USDC/ADX rewards
- new instruction to revoke freeze authority for ADX/ALP mints

## 1.0.3 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/9YyYNWvHyzBzWRESq4dQV92uN7DJ4PsTCSX1Wq4kEMWn)

### Fixed

- fix a bug in the swap where some account would not be reloaded causing the IX to fail due to overflow in calculations (impact = minor fee miscalculation ~100$)

## 1.0.2 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/G74hkR9wRQKJ6iUebQYKBGWkUwpwZBAkYN4CEr22tfgo)

### Fixed

- fix a bug in the get_lp_price view where it was returning in USDC decimals instead of the new PRICE decimals (minor)
- fix a bug where AddCollateralLong was using the Oracle instead of TradeOracle (the IX was returning error, minor)

## 1.0.1 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/6g7Za5WtnR69qvMW2C4iNDzJTHsLYk1WFbNsSDdFfA4B)

### Fixed

- #128 fix a bug when valuing the collateral for leverage in several IX (in the case of dual oracle custodies)

## 1.0.0 [Released](https://dao.adrena.xyz/dao/AdrenaDAO/proposal/8ycGA4g6ZBrtJHZZwjJiuu9mHBiVx646uLkcJbUxLaQe)
