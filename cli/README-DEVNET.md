# CLI GUIDE

## Introduction

This guide cover the setup of an Adrena program onchain from scratch and the interaction with existing Adrena program using CLI.

## Prepare

### Install tools

```
> node -v
v1.18.12
> cd ./app
> npm i
> cd ..
> anchor --version
anchor-cli 0.29.0
> solana --version
solana-cli 1.18.12 (src:c78a9720; feat:3469865029, client:SolanaLabs)
```

### Create CURRENT-DEPLOYMENT.md file

`CURRENT-DEPLOY-TEMPLATE-DEVNET.md` is one way to store key information regarding Adrena program deployment. We do recommend you to use it.

```bash
cp ./cli/CURRENT-DEPLOYMENT-TEMPLATE-DEVNET.md ./CURRENT-DEPLOYMENT-DEVNET.md
```

Along the way, please do fill the file to keep.

### Generate Admin keypair

Creates an admin account to rule over the Adrena program and save it somewhere.
In the examples bellow, we consider all the keypairs to be saved in: `./adrena-keypairs/devnet/` directory.

```bash
solana-keygen new --outfile ./adrena-keypairs/devnet/admin.json

# Airdrop some SOL so the admin wallet can pay fees
# Note: you may have to wait due to Rate limit, to unblock yourself, you may want to
# send SOL manually to the new address
solana airdrop 1 <RECIPIENT_PUBLIC_KEY> --url https://api.devnet.solana.com
```

### [Optional] Generate mint keypairs

In case you deploy on devnet, you may want to create mints to use in custodies.

#### Create a new mint authority

```bash
solana-keygen new --outfile ./adrena-keypairs/devnet/<TOKEN NAME>.json

# Airdrop some SOL so the mint authority wallet can pay fees
# Note: you may have to wait due to Rate limit, to unblock yourself, you may want to
# send SOL manually to the new address
solana airdrop 1 <RECIPIENT_PUBLIC_KEY> --url https://api.devnet.solana.com

# Example
solana-keygen new --outfile ./adrena-keypairs/devnet/jitoSOL.json
```

#### Create a new mint

```bash
spl-token create-token --decimals 6 --mint-authority ./adrena-keypairs/devnet/<TOKEN NAME>.json
```

#### Mint tokens

```bash
# Create local ATA for new token
spl-token create-account <TOKEN_ADDRESS>

# Mint new tokens
spl-token mint <TOKEN_ADDRESS> <AMOUNT> --mint-authority ./adrena-keypairs/devnet/<TOKEN_NAME>.json

# Examples
spl-token mint DmfSVHxadyJU4HJXT4pvXMzVfBHDiyS32NRKSAdxkzEy 50000000000 --mint-authority ./adrena-keypairs/devnet/jitoSOL.json
```

#### [Optional] Transfer tokens to devnet faucet bank

```bash
spl-token transfer <TOKEN_ADDRESS> <AMOUNT> <DEVNET_BANK> --fund-recipient

# Example
spl-token transfer DmfSVHxadyJU4HJXT4pvXMzVfBHDiyS32NRKSAdxkzEy 500000 AQGXYGyu5zCU3e1MLegTEW32bXRNfdcqQiYwEUQub1gA --fund-recipient
```

#### [Optional] Create users to provide liquidity

```bash
solana-keygen new --outfile ./adrena-keypairs/devnet/<NAME>.json

# Airdrop some SOL so the user can pay fees
# Note: you may have to wait due to Rate limit, to unblock yourself, you may want to
# send SOL manually to the new address
solana airdrop 1 <RECIPIENT_PUBLIC_KEY> --url https://api.devnet.solana.com
```

## Deploy a new Adrena program from scratch

### Generate a new program id

```bash
./scripts/change_program_id.sh
```

### Deploy the program

```bash
anchor build
anchor deploy --program-name adrena --provider.cluster devnet --program-keypair ./target/deploy/adrena-keypair.json
```

If bug:

```bash
solana-keygen recover -o DEPLOY_KEYPAIR
solana program write-buffer ./target/deploy/adrena.so --buffer DEPLOY_KEYPAIR --fee-payer ./adrena-keypairs/devnet/admin.json --buffer-authority ./adrena-keypairs/devnet/admin.json --url <DEVNET_RPC>

solana program deploy \      
  --program-id ./target/deploy/adrena-keypair.json \
  --buffer 6rQgduE8vnSwdZr1abaYFyz4X9xDLQd4naqc1vbn3HRU \
  --upgrade-authority ./adrena-keypairs/devnet/admin.json \
  --fee-payer ./adrena-keypairs/devnet/admin.json \
  --url <DEVNET_RPC>
```

NOTE: if you experience an error like:

```
Error: Account AqJVpVbjJae8sfY8SooA5qzjHmoZusB1HnidEd16EDeH has insufficient funds for spend (19.27299864 SOL) + fee (0.0137 SOL)
```

you need to fund the local account used to deploy the program.

NOTE: if need to update the program and experiencing an error like:

```
Error: Deploying program failed: RPC response error -32002: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction [3 log messages]
```

you need to manually extend the program data account space

```bash
solana program extend <PROGRAM_ID> 200000 -u d
```

### Deploy the program IDL

```bash
# First time
anchor idl init --filepath ./target/idl/adrena.json --provider.cluster devnet <PROGRAM_ID>

# nth time
anchor idl upgrade --filepath ./target/idl/adrena.json --provider.cluster devnet <PROGRAM_ID>
```

### Chose a governance realm name

Chose the name of the governance's realm that will be created. Must be unique.

### Get the governance realm pda

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-dao-realm-key --name <REALM_NAME>

# example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json get-dao-realm-key --name AdrenaDaoTestingZ
```

### Initialize the Adrena program

#### Init core 1/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-one-core\
 --fee-redistribution-mint <FEE_REDISTRIBUTION_MINT>\
 --protocol-fee-recipient <PROTOCOL_FEE_RECIPIENT_TOKEN_ACCOUNT>\
 --core-contributor-bucket-allocation <ALLOCATION>\
 --foundation-bucket-allocation <ALLOCATION>\
 --ecosystem-bucket-allocation <ALLOCATION>

# Example
# Total ADX supply: 1000000000
# Core contributor: 36% => 360000000
# Foundation: 9% => 90000000
# Ecosystem: 55% => 550000000
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-one-core\
 --fee-redistribution-mint 3jdYcGYZaQVvcvMQGqVpt37JegEoDDnX7k4gSGAeGRqG\
 --protocol-fee-recipient 5STGJRnjLKbssEkk5AmKpqebPLt5yk71RMFmGtxWwjgG\
 --core-contributor-bucket-allocation 360000000\
 --foundation-bucket-allocation 90000000\
 --ecosystem-bucket-allocation 550000000
```

#### Init LM token metadata 2/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-two-lm-token-metadata

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-two-lm-token-metadata
```

#### Init governance 3/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-three-governance --governance-realm <GOVERNANCE_REALM>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-three-governance --governance-realm 9SoFcgcpz67JvHS6PCQZpq35VNfwsuDkjYv1N2Cy5WFP
```

#### Init vesting 4/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-four-vesting

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-four-vesting
```

#### Init lm staking one 1/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lm-staking-one

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lm-staking-one
```

#### Init lm staking two 2/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lm-staking-two

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lm-staking-two
```

#### Init lm staking two 3/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lm-staking-three

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lm-staking-three
```

#### Init lm staking two 4/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lm-staking-four

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lm-staking-four
```
### Initialize LP staking

#### Init lp staking one 1/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lp-staking-one <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lp-staking-one main-pool
```

#### Init lp staking two 2/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lp-staking-two <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lp-staking-two main-pool
```

#### Init lp staking three 3/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lp-staking-three <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lp-staking-three main-pool
```

#### Init lp staking four 4/4

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-lp-staking-four <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-lp-staking-four main-pool
```

### Create the governance realm

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> create-dao-realm \
--name <REALM_NAME> \
--min-community-weight-to-create-governance <WEIGHT>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json create-dao-realm \
--name AdrenaDaoTestingZ \
--min-community-weight-to-create-governance 1000000
```

You may access the new realm here:

```bash
https://app.realms.today/dao/<REALM_KEY>?cluster=devnet

# Example
https://app.realms.today/dao/8v8d88EQB9MeXNMhK4xcYQCbMgjn7BjkGZkneecXiFRK?cluster=devnet
```

### [Optional] Verify Cortex

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-cortex

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json get-cortex
```

### Setup a new Pool

#### Pool initialization

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> add-pool-part-one <POOL_NAME> \
  <AUM_SOFT_CAP_USD> \
  <LP_TOKEN_NAME> \
  <LP_TOKEN_SYMBOL> \
  <LP_TOKEN_URI>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-pool-part-one main-pool \
10000000000000 \
"Adrena LP Token" \
"ALP" \
"https://arweave.net/fRrkJvBTj9ZMa3j1sy5HMEvBNVIP0MDfonXXyKfbSW4"
```

then:

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> add-pool-part-two <POOL_NAME> \
 --genesis-lock-campaign-duration <NUMBER_IN_SECONDS> \
 --genesis-reserved-grant-duration <NUMBER_IN_SECONDS> \
 --genesis-lock-campaign-start-date <TIMESTAMP>

# Example
# 604800 = 7 days
# 259200 = 3 days
# 3600 = 1 hour
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-pool-part-two main-pool \
 --genesis-lock-campaign-duration 3600 \
 --genesis-reserved-grant-duration 200 \
 --genesis-lock-campaign-start-date 1726242947
```

#### Check pool info

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-pool <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json get-pool main-pool
```

#### Check LP token mint

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-lp-token-mint <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json get-lp-token-mint main-pool
```

#### Init oracle account

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> init-oracle \
 --prices '[{"name": "SOLUSD", "chaos_labs_feed_id": 0}, {"name": "JITOSOLUSD", "chaos_labs_feed_id": 1}, {"name": "BTCUSD", "chaos_labs_feed_id": 2}, {"name": "WBTCUSD", "chaos_labs_feed_id": 3}, {"name": "BONKUSD", "chaos_labs_feed_id": 4}, {"name": "USDCUSD", "chaos_labs_feed_id": 5}, {"name": "ETHUSD", "chaos_labs_feed_id": 6}]'

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json init-oracle \
 --prices '[{"name": "SOLUSD", "chaos_labs_feed_id": 0}, {"name": "JITOSOLUSD", "chaos_labs_feed_id": 1}, {"name": "BTCUSD", "chaos_labs_feed_id": 2}, {"name": "WBTCUSD", "chaos_labs_feed_id": 3}, {"name": "BONKUSD", "chaos_labs_feed_id": 4}, {"name": "USDCUSD", "chaos_labs_feed_id": 5}, {"name": "ETHUSD", "chaos_labs_feed_id": 6}]'
```

#### Add custodies

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> add-custody <POOL_NAME> -t pyth [--stablecoin] <TOKEN_MINT> <TOKEN_ORACLE_ACCOUNT> --max-cumulative-short-position-size-usd <MAX_SIZE>

NOTE: USDC should always be the first custody added.

# Example
# USDC
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-custody main-pool --stablecoin 3jdYcGYZaQVvcvMQGqVpt37JegEoDDnX7k4gSGAeGRqG USDCUSD USDCUSD --max-cumulative-short-position-size-usd 0

# BTC
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-custody main-pool 7MoYkgWVCEDtNR6i2WUH9LTUSFXkQCsD9tBHriHQvuP5 WBTCUSD BTCUSD --max-cumulative-short-position-size-usd 1000000

# BONK
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-custody main-pool 2eU7sUxhpQuBaUrjd6oPTzoFZNPEaawrAka4zqowMzbJ BONKUSD BONKUSD --max-cumulative-short-position-size-usd 250000

# jitoSOL
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-custody main-pool DmfSVHxadyJU4HJXT4pvXMzVfBHDiyS32NRKSAdxkzEy JITOSOLUSD SOLUSD --max-cumulative-short-position-size-usd 1000000
```

### Create DAO Governance

```bash
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json create-dao-governance --realm-name AdrenaDaoTestingZ --min-tokens-to-create-proposal 1000000
```

### Create DAO Native Treasury

```bash
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json create-dao-native-treasury <GOVERNANCE_PUBKEY>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json create-dao-native-treasury 9SoFcgcpz67JvHS6PCQZpq35VNfwsuDkjYv1N2Cy5WFP
```

### Transfer pool upgrade authority to DAO governance (If deploying on governance)

```bash
solana program set-upgrade-authority <PROGRAM_ID> \
  --new-upgrade-authority <NATIVE_TREASURY_PUBKEY> \
  --skip-new-upgrade-authority-signer-check

# Example
solana program set-upgrade-authority 3wgAScGvh6Wbq42bSDdJru6EemY6HuzKMXuFRs9Naev9 \
  --new-upgrade-authority AHRGRFzkqX1nfqEb66RZ9UeWCpvxgn3dio1LehvyCbxM \
  --skip-new-upgrade-authority-signer-check
```

### Transfer program authority to DAO native treasury (If deploying on governance)

After this command, you won't have access to permissioned instructions through CLI. You'll have to create DAO proposals.

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-admin <NATIVE_TREASURY>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-admin AHRGRFzkqX1nfqEb66RZ9UeWCpvxgn3dio1LehvyCbxM
```

### Create at least founder vesting so you are able to create votes

Can use the following website to find timestamp: https://www.unixtimestamp.com/

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> add-vest --beneficiary-wallet <WALLET> --amount <LM_TOKEN_AMOUNT> --unlock-start-timestamp <UNIX_TIMESTAMP> --unlock-end-timestamp <UNIX_TIMESTAMP>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-vest --beneficiary-wallet 95f5JT9hfCV9a5sivpgQeXSAiVeXjC1aMyxHqGb7oAPf --amount 3350000 --unlock-start-timestamp 1717338330 --unlock-end-timestamp 1780410330 --vote-multiplier 40000

ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-vest --beneficiary-wallet AQGXYGyu5zCU3e1MLegTEW32bXRNfdcqQiYwEUQub1gA --amount 100000000 --unlock-start-timestamp 1717338330 --unlock-end-timestamp 1780410330 --vote-multiplier 40000
```

### Transfer realm authority to DAO native treasury (If deploying on governance)

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-dao-realm-authority --realm <REALM> --new-realm-authority <NATIVE_TREASURY>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-dao-realm-authority --realm 8v8d88EQB9MeXNMhK4xcYQCbMgjn7BjkGZkneecXiFRK --new-realm-authority AHRGRFzkqX1nfqEb66RZ9UeWCpvxgn3dio1LehvyCbxM
```

### Finalize Genesis Lock Campaign

Finalize manually the Genesis Lock Campaign after time is over (which is not supported anymore - for testing only)

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> finalize-genesis-lock-campaign <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json finalize-genesis-lock-campaign main-pool
```

## Commands below are to be executed via CLI or via DAO proposals

### Activate the pool

#### Set pool liquidity state

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-pool-liquidity-state <POOL_NAME> <STATE>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-pool-liquidity-state main-pool active
```

#### Allow swap on the pool [Optional]

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-pool-allow-swap <POOL_NAME> <ALLOW>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-pool-allow-swap main-pool true
```

#### Allow trade on the pool [Optional]

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-pool-allow-trade <POOL_NAME> <ALLOW>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-pool-allow-trade main-pool true
```

#### Add liquidity to custodies

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> add-liquidity <POOL_NAME> <TOKEN_MINT> --amount-in <AMOUNT_IN> --min-amount-out <AMOUNT_OUT>

# Example
# ~10m
# USDC 30% => $3m => 3m USDC => 3000000000000 USDC
# BTC 20% => $2m => 55250,22 USD / BTC => 36198950 BTC
# BONK 5% => $500k => 0.00001648 USD / BONK => 3033980582500000 BONK
# jitoSOL 45% => $4.5m => 147.16 USD / jitoSOL => 30578961674400 jitoSOL

# Add USDC liquidity
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-liquidity main-pool 3jdYcGYZaQVvcvMQGqVpt37JegEoDDnX7k4gSGAeGRqG --amount-in 3000000000000 --min-amount-out 0

# Add BTC liquidity
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-liquidity main-pool 7MoYkgWVCEDtNR6i2WUH9LTUSFXkQCsD9tBHriHQvuP5 --amount-in 36198950 --min-amount-out 0

# Add BONK liquidity
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-liquidity main-pool 2eU7sUxhpQuBaUrjd6oPTzoFZNPEaawrAka4zqowMzbJ --amount-in 3033980582500000 --min-amount-out 0

# Add jitoSOL liquidity
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json add-liquidity main-pool DmfSVHxadyJU4HJXT4pvXMzVfBHDiyS32NRKSAdxkzEy --amount-in 30578961674400 --min-amount-out 0
```

#### Check custodies

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-custodies <POOL_NAME>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json get-custodies main-pool
```

#### Claim vest

```bash
ts-node cli/src/cli.ts -k <VEST_WALLET_KEYPAIR> claim-vest

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/orex-work.json claim-vest
```

### Change pool ratios

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-custodies-ratios <POOL_NAME> <TARGET_CUSTODY_N> <MIN_CUSTODY_N> <MAX_CUSTODY_N>

# Example
# Restrained ratios
# USDC / BTC / BONK / jitoSOL
# USDC 30%
# BTC 20%
# BONK 5%
# jitoSOL 45%
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-custodies-ratios main-pool \
2700 3000 3300 \
1800 2000 2200 \
450 500 550 \
4050 4500 4950

# USDC / BTC / BONK / SOL
# Full range ratios
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-custodies-ratios main-pool \
2500 0 10000 \
2500 0 10000 \
2500 0 10000 \
2500 0 10000
```

### Liquidator run

Liquidator script run liquidations for one custody at a time.

TOKEN_MINT: the token mint of the custody you wanna run liquidations for.

```bash
ts-node cli/src/liquidator.ts -k <LIQUIDATOR_KEYPAIR> run <POOL_NAME> <TOKEN_MINT>

# Example
# BONK positions liquidation
ts-node cli/src/liquidator.ts -k ./adrena-keypairs/devnet/liquidator.json run main-pool 3AHAG1ZSUnPz43XBFKRqnLwhdyz29WhHvYQgVrcheCwr

# BTC positions liquidation
ts-node cli/src/liquidator.ts -k ./adrena-keypairs/devnet/liquidator.json run main-pool HRvpfs8bKiUbLzSgT4LmKKugafZ8ePi5Vq7icJBC9dnM

# jitoSOL positions liquidation
ts-node cli/src/liquidator.ts -k ./adrena-keypairs/devnet/liquidator.json run main-pool DmfSVHxadyJU4HJXT4pvXMzVfBHDiyS32NRKSAdxkzEy
```

### Set Pool AUM Soft Limit

```bash
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-pool-aum-soft-cap-usd <POOL_NAME> <AUM_SOFT_CAP>

# Example
ts-node cli/src/cli.ts -k ./adrena-keypairs/devnet/admin.json set-pool-aum-soft-cap-usd main-pool 12000000
```

## Troubleshooting

```bash
Error: IDL for program `adrena` does not have `metadata.address` field.
```

Check `target/idl/adrena.json` to have the following at the end of the file:

````json
  "metadata": {
    "address": "3UT4rMBgSTi6NPHVYKM5AxaWgrkGNJeFQED8NK86axk3"
  }
```

```bash
Error Message: Fallback functions are not supported.
````

Check that you have deployed the program correctly, with the correct program ID.

```bash
Error: Deploying program failed: RPC response error -32002: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction [3 log messages]
```

Execute

```bash
solana program extend G6rjNVUhVDbexZVpR1SeFHW9bx3o8PyDnB5XpbmwGaSY 200000 -u d
```
