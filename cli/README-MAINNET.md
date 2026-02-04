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

`CURRENT-DEPLOY-TEMPLATE-MAINNET.md` is one way to store key information regarding Adrena program deployment. We do recommend you to use it.

```bash
cp ./cli/CURRENT-DEPLOYMENT-TEMPLATE-MAINNET.md ./CURRENT-DEPLOYMENT-MAINNET.md
```

Along the way, please do fill the file to keep.

### Generate an Admin/Upgrade Authority keypair

Creates an admin account to rule over the Adrena program and save it somewhere.
In the examples bellow, we consider all the keypairs to be saved in: `./adrena-keypairs/mainnet/` directory.

```bash
solana-keygen new --outfile ./adrena-keypairs/mainnet/admin.json
```

Then fund it with enough SOL to be able to deploy the Adrena program.

## Deploy a new Adrena program from scratch

### Close an old program

```bash
solana program close G6rjNVUhVDbexZVpR1SeFHW9bx3o8PyDnB5XpbmwGaSY --keypair ./adrena-keypairs/mainnet/admin.json --bypass-warning
```

### Generate a new program id

```bash
./scripts/change_program_id.sh
```

### Deploy the program

```bash
anchor build
anchor deploy --program-name adrena --provider.cluster mainnet --program-keypair ./target/deploy/adrena-keypair.json --provider.wallet <UPGRADE_AUTHORITY>

# Example
anchor deploy --program-name adrena --provider.cluster mainnet --program-keypair ./target/deploy/adrena-keypair.json --provider.wallet ./adrena-keypairs/mainnet/admin.json
```

NOTE: if you experience an error like:

```
Error: Account AqJVpVbjJae8sfY8SooA5qzjHmoZusB1HnidEd16EDeH has insufficient funds for spend (19.27299864 SOL) + fee (0.0137 SOL)
```

you need to fund the account used to deploy the program.

NOTE: if need to update the program and experiencing an error like:

```
Error: Deploying program failed: RPC response error -32002: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction [3 log messages]
```

you need to manually extend the program data account space

```bash
solana program extend <PROGRAM_ID> 1450000 -k ./adrena-keypairs/mainnet/admin.json

# Example
solana program extend 13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet 1450000 -k ./adrena-keypairs/mainnet/admin.json
```

If bug:

```bash
solana-keygen recover -o DEPLOY_KEYPAIR
solana program deploy --buffer DEPLOY_KEYPAIR ./target/deploy/adrena.so --fee-payer ./adrena-keypairs/mainnet/admin.json --upgrade-authority ./adrena-keypairs/mainnet/admin.json --url <MAINNET_RPC>
```

If Deploying Buffer Bug:

```bash
solana-keygen recover -o DEPLOY_KEYPAIR
solana program write-buffer ./target/deploy/adrena.so --buffer DEPLOY_KEYPAIR --fee-payer ./adrena-keypairs/mainnet/admin.json --buffer-authority ./adrena-keypairs/mainnet/admin.json --url <MAINNET_RPC>
```

If transferring buffer authority to DAO wallet:

```bash
solana program set-buffer-authority --new-buffer-authority 7VzEXYvGmLg3tdVuFuGFQdr7GP5tutTUt8EcTGHvG8Ev <BUFFER> --buffer-authority ./adrena-keypairs/mainnet/admin.json -k ./adrena-keypairs/mainnet/admin.json
 --url <MAINNET_RPC>
```

### Deploy the program IDL

```bash
# First time
anchor idl init --filepath ./target/idl/adrena.json --provider.cluster mainnet <PROGRAM_ID> --provider.wallet <UPGRADE_AUTHORITY>

# Example
anchor idl init --filepath ./target/idl/adrena.json --provider.cluster mainnet 13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet --provider.wallet ./adrena-keypairs/mainnet/admin.json

# nth time
anchor idl upgrade --filepath ./target/idl/adrena.json --provider.cluster mainnet <PROGRAM_ID> --provider.wallet <UPGRADE_AUTHORITY>
```

### Chose a governance realm name

Chose the name of the governance's realm that will be created. Must be unique.

### Get the governance realm pda

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> get-dao-realm-key --name <REALM_NAME>

# example
ts-node cli/src/cli.ts -k ./adrena-keypairs/mainnet/admin.json get-dao-realm-key -u <MAINNET_RPC> --name AdrenaDAO
```

### Initialize the Adrena program

#### Init core 1/4

```bash
ts-node cli/src/cli.ts -u mainnet -k <ADMIN_KEYPAIR> init-one-core\
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
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-one-core\
 --fee-redistribution-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v\
 --protocol-fee-recipient 2m1kmmmHNm3gcdsoY9HHS96Fbsp8v2ghdm8dZW8Pi7vM\
 --core-contributor-bucket-allocation 360000000\
 --foundation-bucket-allocation 90000000\
 --ecosystem-bucket-allocation 550000000
```

#### Init LM token metadata 2/4

> [!IMPORTANT]  
> Look at the instruction to make sure the name of the token is correct. Either for test mainnet or for real mainnet.

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-two-lm-token-metadata

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-two-lm-token-metadata
```

#### Init governance 3/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-three-governance --governance-realm <GOVERNANCE_REALM>

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-three-governance --governance-realm GWe1VYTRMujAtGVhSLwSn4YPsXBLe5qfkzNAYAKD44Nk
```

#### Init vesting 4/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-four-vesting

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-four-vesting
```

#### Init lm staking one 1/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lm-staking-one

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lm-staking-one
```

#### Init lm staking two 2/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lm-staking-two

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lm-staking-two
```

#### Init lm staking two 3/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lm-staking-three

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lm-staking-three
```

#### Init lm staking two 4/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lm-staking-four

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lm-staking-four
```

### Create the governance realm

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> create-dao-realm \
--name <REALM_NAME> \
--min-community-weight-to-create-governance <WEIGHT>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json create-dao-realm \
--name AdrenaDAO \
--min-community-weight-to-create-governance 10000000
```

You may access the new realm here:

```bash
https://app.realms.today/dao/<REALM_KEY>

# Example
https://app.realms.today/dao/GWe1VYTRMujAtGVhSLwSn4YPsXBLe5qfkzNAYAKD44Nk
```

### [Optional] Verify Cortex

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> get-cortex

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json get-cortex
```

### Setup a new Pool

#### Pool initialization

> [!IMPORTANT]  
> Make sure the name of the token in the command line is correct. Either for test mainnet or for real mainnet.

> [!IMPORTANT]  
> Make sure the list of wallets for the genesis reserved spot are correct in the code.

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-pool-part-one <POOL_NAME> \
  <AUM_SOFT_CAP_USD> \
  <LP_TOKEN_NAME> \
  <LP_TOKEN_SYMBOL> \
  <LP_TOKEN_URI>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-pool-part-one main-pool \
10000000000000 \
"Adrena LP Token" \
"ALP" \
"https://arweave.net/fRrkJvBTj9ZMa3j1sy5HMEvBNVIP0MDfonXXyKfbSW4"
```

then:

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-pool-part-two <POOL_NAME> \
 --genesis-lock-campaign-duration <NUMBER_IN_SECONDS> \
 --genesis-reserved-grant-duration <NUMBER_IN_SECONDS> \
 --genesis-lock-campaign-start-date <TIMESTAMP>

 # Example
 # 259200 = 3 days
 # 172800 = 2 days
 # 86400 = 1 days
 # 3600 = 1 hour
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-pool-part-two main-pool \
 --genesis-lock-campaign-duration 259200 \
 --genesis-reserved-grant-duration 86400 \
 --genesis-lock-campaign-start-date 1726574400
```

#### Check pool info

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> get-pool <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json get-pool main-pool
```

#### Check LP token mint

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> get-lp-token-mint <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json get-lp-token-mint main-pool
```

#### Add custodies

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-custody <POOL_NAME> -t pyth [--stablecoin] <TOKEN_MINT> <TOKEN_ORACLE_ACCOUNT> --max-cumulative-short-position-size-usd <MAX_SIZE>

NOTE: USDC should always be the first custody added.

# Example
# USDC
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-custody main-pool --stablecoin EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX --max-cumulative-short-position-size-usd 0

# WBTC TODO: re-add later
# ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-custody main-pool 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh 9gNX5vguzarZZPjTnE1hWze3s6UsZ7dsU3UnAmKPnMHG 4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo --max-cumulative-short-position-size-usd 1000000

# BONK
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-custody main-pool DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 DBE3N8uNjhKPRHfANdwGvCZghWXyLPdqdSbEW2XFwBiX DBE3N8uNjhKPRHfANdwGvCZghWXyLPdqdSbEW2XFwBiX --max-cumulative-short-position-size-usd 250000

# jitoSOL
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-custody main-pool J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn AxaxyeDT8JnWERSaTKvFXvPKkEdxnamKSqpWbsSjYg1g 7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE --max-cumulative-short-position-size-usd 1000000
```

### Initialize LP staking

#### Init lp staking one 1/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lp-staking-one <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lp-staking-one main-pool

ts-node cli/src/cli.ts -u https://adrena-solanam-6f0c.mainnet.rpcpool.com/73265fca-9d3a-41fa-9e80-13edacf6526d -k ./adrena-keypairs/mainnet/admin.json update-pool-aum main-pool
```

#### Init lp staking two 2/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lp-staking-two <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lp-staking-two main-pool
```

#### Init lp staking three 3/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lp-staking-three <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lp-staking-three main-pool
```

#### Init lp staking four 4/4

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> init-lp-staking-four <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json init-lp-staking-four main-pool
```

### Change pool ratios

````bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> set-custodies-ratios <POOL_NAME> <TARGET_CUSTODY_N> <MIN_CUSTODY_N> <MAX_CUSTODY_N>

# Example
# Restrained ratios
# USDC / BTC / BONK / jitoSOL
# USDC 30%
# BTC 20%
# BONK 5%
# jitoSOL 45%
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-custodies-ratios main-pool \
2000 3000 4000 \
1000 2000 3000 \
250 500 750 \
3500 4500 5500

ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-custodies-ratios main-pool \
3000 5000 7000 \
250 500 750 \
3500 4500 5500

### Create DAO Governance

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json create-dao-governance --realm-name AdrenaDAO --min-tokens-to-create-proposal 5000000
````

### Create DAO Native Treasury

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json create-dao-native-treasury <GOVERNANCE_PUBKEY>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json create-dao-native-treasury HgeoVqTTMQ9K5GZAUpPKaz5PS8Rn55yR5e5SwmB3DbKB
```

### Create at least founder vesting so you are able to create votes

Can use the following website to find timestamp: https://www.unixtimestamp.com/

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-vest --beneficiary-wallet <WALLET> --amount <LM_TOKEN_AMOUNT> --unlock-start-timestamp <UNIX_TIMESTAMP> --unlock-end-timestamp <UNIX_TIMESTAMP>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet 95f5JT9hfCV9a5sivpgQeXSAiVeXjC1aMyxHqGb7oAPf --amount 3350000 --unlock-start-timestamp 1717338330 --unlock-end-timestamp 1780410330 --vote-multiplier 40000

ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet AQGXYGyu5zCU3e1MLegTEW32bXRNfdcqQiYwEUQub1gA --amount 100000000 --unlock-start-timestamp 1734247958 --unlock-end-timestamp 1789457558 --vote-multiplier 40000
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet 5Wc6Xjsd68MY1iwsiLFtEcCNEE8scBhKu2mGyYiyC6ux --amount 100000000 --unlock-start-timestamp 1734247958 --unlock-end-timestamp 1789457558 --vote-multiplier 40000


ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet 6iQqd2L4RNWRTbAgZPGxhNRtERqZJ1gNhcfsCFGvbPdK --amount 3200000 --unlock-start-timestamp 1734247958 --unlock-end-timestamp 1789457558 --vote-multiplier 40000
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet 64cXm3C66UkKvNwQFxV8Vg7SCcPyvsay826J7QeFFQuH --amount 1800000 --unlock-start-timestamp 1734247958 --unlock-end-timestamp 1789457558 --vote-multiplier 40000
```

### Transfer upgrade authority to DAO governance (If deploying on governance)

```bash
solana program set-upgrade-authority <PROGRAM_ID> \
  --new-upgrade-authority <NATIVE_TREASURY_PUBKEY> \
  --skip-new-upgrade-authority-signer-check -k <ADMIN_KEYPAIR>

# Example
solana program set-upgrade-authority 13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet \
  --new-upgrade-authority 7VzEXYvGmLg3tdVuFuGFQdr7GP5tutTUt8EcTGHvG8Ev \
  --skip-new-upgrade-authority-signer-check -k ./adrena-keypairs/mainnet/admin.json
```

### Transfer program authority to DAO native treasury (If deploying on governance)

After this command, you won't have access to permissioned instructions through CLI. You'll have to create DAO proposals.

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> set-admin <NATIVE_TREASURY>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-admin 7VzEXYvGmLg3tdVuFuGFQdr7GP5tutTUt8EcTGHvG8Ev
```

### Transfer realm authority to DAO native treasury (If deploying on governance)

```bash
ts-node cli/src/cli.ts -k <ADMIN_KEYPAIR> set-dao-realm-authority --realm <REALM> --new-realm-authority <NATIVE_TREASURY>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-dao-realm-authority --realm 65M6EkpcQ5bXfJBhkmgNT3gUTB2YtW5tmBbsDEP6Gfcj --new-realm-authority Jtu9H2FMtQVmcUytkaEmCcQt3XS5TdjqBhJtXTYJS8K
```

#### Finalize Genesis Lock Campaign (Optional)

Genesis lock campaign should finalize by itself through Sablier thread trigger. In case it's not happening due to any reason dev related, you may need to force it.

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> finalize-genesis-lock-campaign <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json finalize-genesis-lock-campaign main-pool
```

## Commands below are to be executed via CLI or via DAO proposals

## Create Proposal for OUT 70% of the USDC of the pool

### After GenesisLock, activate the pool

#### Set pool liquidity state

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> set-pool-liquidity-state <POOL_NAME> <STATE>

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-pool-liquidity-state main-pool active
```

#### Allow swap on the pool [Optional]

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> set-pool-allow-swap <POOL_NAME> <ALLOW>

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-pool-allow-swap main-pool true
```

#### Allow trade on the pool [Optional]

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> set-pool-allow-trade <POOL_NAME> <ALLOW>

 # Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json set-pool-allow-trade main-pool true
```

#### Allow add liquidity on the pool [Optional]

#### Add liquidity to custodies

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-liquidity <POOL_NAME> <TOKEN_MINT> --amount-in <AMOUNT_IN> --min-amount-out <AMOUNT_OUT>

# Example
# ~10m
# USDC 30% => $3m => 3m USDC => 3000000000000 USDC
# BTC 64.999% => $6.499900m => 56,743.40 USD / BTC => 114549004 BTC
# BONK 5% => $500k => 0.00001654 USD / BONK => 3022974607000000 BONK
# SOL 0.01% => $1k => 127.58 USD / SOL => 7838219156 SOL

# Add USDC liquidity
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-liquidity main-pool 3jdYcGYZaQVvcvMQGqVpt37JegEoDDnX7k4gSGAeGRqG --amount-in 3000000000000 --min-amount-out 0

# Add BTC liquidity
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-liquidity main-pool 7MoYkgWVCEDtNR6i2WUH9LTUSFXkQCsD9tBHriHQvuP5 --amount-in 114549004 --min-amount-out 0

# Add BONK liquidity
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-liquidity main-pool 4kUrHxiMfeKPGDi6yFV7kte8JjN3NG3aqG7bui4pfMqz --amount-in 3022974607000000 --min-amount-out 0

# Add SOL liquidity
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-liquidity main-pool So11111111111111111111111111111111111111112 --amount-in 7838219156 --min-amount-out 0
```

#### Check custodies

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> get-custodies <POOL_NAME>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json get-custodies main-pool
```

### Setup vesting

#### Create a new vest

Can use the following website to find timestamp: https://www.unixtimestamp.com/

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <ADMIN_KEYPAIR> add-vest --beneficiary-wallet <WALLET> --amount <LM_TOKEN_AMOUNT> --unlock-start-timestamp <UNIX_TIMESTAMP> --unlock-end-timestamp <UNIX_TIMESTAMP>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet 95f5JT9hfCV9a5sivpgQeXSAiVeXjC1aMyxHqGb7oAPf --amount 3350000 --unlock-start-timestamp 1717338330 --unlock-end-timestamp 1780410330 --vote-multiplier 40000

ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json add-vest --beneficiary-wallet AQGXYGyu5zCU3e1MLegTEW32bXRNfdcqQiYwEUQub1gA --amount 3350000 --unlock-start-timestamp 1717338330 --unlock-end-timestamp 1780410330 --vote-multiplier 40000
```

#### Claim vest

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k <VEST_WALLET_KEYPAIR> claim-vest

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/orex-work.json claim-vest
```

### Create DAO Governance

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json create-dao-governance --realm-name Adaorenareturn2 --min-tokens-to-create-proposal 50000000000
```

### Liquidator run

Liquidator script run liquidations for one custody at a time.

TOKEN_MINT: the token mint of the custody you wanna run liquidations for.

```bash
ts-node cli/src/liquidator.ts -u <MAINNET_RPC> -k <LIQUIDATOR_KEYPAIR> run <POOL_NAME> <TOKEN_MINT>

# Example
# BONK positions liquidation
ts-node cli/src/liquidator.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/liquidator.json run main-pool 3AHAG1ZSUnPz43XBFKRqnLwhdyz29WhHvYQgVrcheCwr

# BTC positions liquidation
ts-node cli/src/liquidator.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/liquidator.json run main-pool HRvpfs8bKiUbLzSgT4LmKKugafZ8ePi5Vq7icJBC9dnM

# SOL positions liquidation
ts-node cli/src/liquidator.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/liquidator.json run main-pool So11111111111111111111111111111111111111112
```

### Set Pool AUM Soft Limit

```bash
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json  set-pool-aum-soft-cap-usd <POOL_NAME> <AUM_SOFT_CAP>

# Example
ts-node cli/src/cli.ts -u <MAINNET_RPC> -k ./adrena-keypairs/mainnet/admin.json  set-pool-aum-soft-cap-usd main-pool 12000000
```

## Troubleshooting

```bash
Error: IDL for program `adrena` does not have `metadata.address` field.
```

Check `target/idl/adrena.json` to have the following at the end of the file:

````json
  "metadata": {
    "address": "13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet"
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
solana program extend G6rjNVUhVDbexZVpR1SeFHW9bx3o8PyDnB5XpbmwGaSY 200000 -u <MAINNET_RPC>
```
