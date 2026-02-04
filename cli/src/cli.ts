import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { AdrenaClient } from "./client";
import { Command } from "commander";
import {
  BorrowRateParams,
  Fees,
  InitOneParams,
  OraclePricesSetup,
  PricingParams,
} from "./types";

const lmTokenMintDecimals = 6;

let client: AdrenaClient;

function initClient(clusterUrl: string, adminKeyPath: string): void {
  process.env.ANCHOR_WALLET = adminKeyPath;
  client = new AdrenaClient(clusterUrl, adminKeyPath);
  client.log("Client Initialized");
}

(async function main() {
  const program = new Command();

  program
    .name("cli.ts")
    .description("CLI to Adrena Program")
    .version("0.1.0")
    .option(
      "-u, --url <string>",
      "URL for Solana's JSON RPC",
      "https://api.devnet.solana.com"
    )
    .requiredOption("-k, --keypair <path>", "Filepath to the admin keypair")
    .hook("preSubcommand", (thisCommand, subCommand) => {
      if (!program.opts().keypair) {
        throw Error("required option '-k, --keypair <path>' not specified");
      }
      initClient(program.opts().url, program.opts().keypair);
      client.log(`Processing command '${thisCommand.args[0]}'`);
    })
    .hook("postAction", () => {
      client.log("Done");
    });

  program
    .command("get-dao-token-mint")
    .description("Print governance token mint")
    .action(async () => {
      client.prettyPrint(client.getGovernanceTokenKey());
    });

  program
    .command("get-dao-realm-key")
    .description("Print governance realm address")
    .requiredOption("-n, --name <string>", "Name of the realm")
    .action((options) => {
      client.prettyPrint(client.getDaoRealmKey(options.name));
    });

  program
    .command("set-dao-realm-authority")
    .description("Change the DAO realm authority")
    .requiredOption("-r, --realm <string>", "Realm pubkey")
    .requiredOption(
      "-n, --new-realm-authority <string>",
      "New authority pubkey"
    )
    .action(async (options) => {
      await client.setRealmAuthority(
        new PublicKey(options["realm"]),
        new PublicKey(options["newRealmAuthority"])
      );
    });

  program
    .command("create-dao-realm")
    .description("Create the governance realm using spl-governance program")
    .requiredOption("-n, --name <string>", "Name of the new realm")
    .requiredOption(
      "-m, --min-community-weight-to-create-governance <int>",
      "Minimum of tokens required to create a new governance"
    )
    .action(async (options) => {
      const realmPubkey = await client.createDaoRealm(
        options["name"],
        new BN(options.minCommunityWeightToCreateGovernance)
      );
    
      console.log(`Realm Pubkey: ${realmPubkey.toBase58()}`);
    });

  program
    .command("create-dao-native-treasury")
    .argument("<string>", "Governance")
    .description("Create a governance on realm using spl-governance program")
    .action(async (governance) => {
      const nativeTreasuryPubkey = await client.createDaoNativeTreasury(
        new PublicKey(governance)
      );
    
      console.log(`Native Treasury Pubkey: ${nativeTreasuryPubkey.toBase58()}`);
    });

  program
    .command("create-dao-governance")
    .description("Create a governance on realm using spl-governance program")
    .requiredOption("-r, --realm-name <string>", "Name of the governance realm")
    .requiredOption(
      "-m, --min-tokens-to-create-proposal <number>",
      "Min tokens to create proposal (native unit)"
    )
    .action(async (options) => {
      const governancePubkey = await client.createDaoGovernance(
        options.realmName,
        new BN(options.minTokensToCreateProposal)
      );
    
      console.log(`Governance Pubkey: ${governancePubkey.toBase58()}`);
    });

  program
    .command("init-one-core")
    .requiredOption(
      "-f, --fee-redistribution-mint <string>",
      "Fee redistribution mint"
    )
    .requiredOption(
      "-r, --protocol-fee-recipient <string>",
      "Protocol fee recipient token account address (must be of same mint as fee redistribution mint)"
    )
    .requiredOption(
      "-c, --core-contributor-bucket-allocation <int>",
      "Core contributors allocation amount"
    )
    .requiredOption(
      "-d, --foundation-bucket-allocation <int>",
      "Foundation bucket allocation amount"
    )
    .requiredOption(
      "-e, --ecosystem-bucket-allocation <int>",
      "Ecosystem allocation amount"
    )
    .description("Initialize the on-chain program (1/4)")
    .action(async (options) => {
      const adrenaConfig: InitOneParams = {
        coreContributorBucketAllocation: new BN(options.coreContributorBucketAllocation).mul(
          new BN(10 ** lmTokenMintDecimals)
        ),
        foundationBucketAllocation: new BN(options.foundationBucketAllocation).mul(
          new BN(10 ** lmTokenMintDecimals)
        ),
        ecosystemBucketAllocation:  new BN(options.ecosystemBucketAllocation).mul(
          new BN(10 ** lmTokenMintDecimals)
        ),
      };
    
      await client.initOneCore(
        new PublicKey(options.feeRedistributionMint),
        new PublicKey(options.protocolFeeRecipient),
        adrenaConfig
      );
    });

  program
    .command("init-two-lm-token-metadata")
    .description("Initialize the on-chain program (2/4)")
    .action(async () => {
      await client.initTwoLmTokenMetadata();
    });

  program
    .command("init-three-governance")
    .description("Initialize the on-chain program (3/4)")
    .requiredOption(
      "-r, --governance-realm <string>",
      "Governance realm address"
    )
    .action(async (options) => {
      await client.initThreeGovernance(new PublicKey(options.governanceRealm));
    });

  program
    .command("init-four-vesting")
    .description("Initialize the on-chain program (4/4)")
    .action(async () => {
      await client.InitFourVesting();
    });

  program
    .command("init-lm-staking-one")
    .description("Initialize staking for given LM token")
    .action(async () => {
      await client.initLmStakingOne();
    });

  program
    .command("init-lm-staking-two")
    .description("Initialize staking for LM token")
    .action(async () => {
      await client.initLmStakingTwo();
    });

  program
    .command("init-lm-staking-three")
    .description("Initialize staking for LM token")
    .action(async () => {
      await client.initLmStakingThree();
    });

  program
    .command("init-lm-staking-four")
    .description("Initialize staking for LM token")
    .action(async () => {
      await client.initLmStakingFour();
    });

  program
    .command("init-lp-staking-one")
    .description("Initialize staking for given LP token mint")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await client.initLpStakingOne(poolName);
    });

  program
    .command("init-oracle")
    .description("Initialize the oracle account")
    .requiredOption(
      "-p, --prices <prices>",
      "JSON string array of oracle prices, e.g. '[{\"name\": \"SOLUSD\", \"chaos_labs_feed_id\": 0}]'"
    )
    .action(async (options) => {
      try {
        const rawPrices = JSON.parse(options.prices);
  
        if (!Array.isArray(rawPrices)) {
          throw new Error("Expected an array of oracle prices");
        }
  
        const oraclePrices: OraclePricesSetup[] = rawPrices.map((p) => {
          if (typeof p.name !== "string" || typeof p.chaos_labs_feed_id !== "number") {
            throw new Error("Each price must have a string 'name' and numeric 'chaos_labs_feed_id'");
          }
  
          return {
            name: {
              value: [...Buffer.from(p.name), ...Array(31 - p.name.length).fill(0)].slice(0, 31),
              length: p.name.length,
            },
            chaosLabsFeedId: p.chaos_labs_feed_id,
          };
        });
  
        await client.initOracle(oraclePrices);
      } catch (err) {
        console.error("❌ Failed to parse or send oracle prices:", err);
        process.exit(1);
      }
    });

  program
    .command("update-pool-aum")
    .description("Update pool AUM")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      console.log(await client.updatePoolAum({
        poolName,
        oraclePrices: await client.getAssetPrices(),
      }));
    });

  program
    .command("disable-tokens-freeze-capabilities")
    .description("Disable tokens freeze capabilities")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      console.log(await client.disableTokensFreezeCapabilities(poolName));
    });

  program
    .command("finalize-genesis-lock-campaign")
    .description("Finalize genesis lock campaign")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await client.finalizeGenesisLockCampaign(poolName);
    });

  program
    .command("init-lp-staking-two")
    .description("Initialize staking for given LP token mint")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await client.initLpStakingTwo(poolName);
    });

  program
    .command("init-lp-staking-three")
    .description("Initialize staking for given LP token mint")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await client.initLpStakingThree(poolName);
    });

  program
    .command("init-lp-staking-four")
    .description("Initialize staking for given LP token mint")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await client.initLpStakingFour(poolName);
    });

  program
    .command("add-vest")
    .description("Add vesting")
    .requiredOption(
      "-w, --beneficiary-wallet <string>",
      "Wallet receiving tokens"
    )
    .requiredOption("-a, --amount <string>", "Token amount")
    .requiredOption(
      "-s, --unlock-start-timestamp <string>",
      "Unlock start timestamp (i.e 1694085185)"
    )
    .requiredOption(
      "-e, --unlock-end-timestamp <string>",
      "Unlock end timestamp (i.e 1694085185)"
    )
    .requiredOption(
      "-v, --vote-multiplier <number>",
      "Vote multiplier in BPS (i.e 10000 = x1, 40000 = x5)"
    )
    .action(async (options) => {
      const txId = await client.addVest(
        new PublicKey(options.beneficiaryWallet),
        new BN(options.amount * 10 ** lmTokenMintDecimals),
        new BN(options.unlockStartTimestamp),
        new BN(options.unlockEndTimestamp),
        options.voteMultiplier
      );

      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("claim-vest")
    .description("Claim vesting for the wallet provided using -k")
    .action(async () => {
      const txId = await client.claimVest();

      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("get-cortex")
    .description("Print cortex global state")
    .action(async () => {
      client.prettyPrint(await client.getCortex());
    });

  program
    .command("add-pool-part-one")
    .description("Create a new pool (part one)")
    .argument("<string>", "Pool name")
    .argument("<number>", "AUM soft cap in USD (native)")
    .argument("<string>", "LP Token name")
    .argument("<string>", "LP Token symbol")
    .argument("<string>", "LP Token URI")
    .action(
      async (
        poolName,
        aumSoftCapUsd,
        lpTokenName,
        lpTokenSymbol,
        lpTokenUri
      ) => {
        await client.addPoolPartOne({
          name: poolName,
          aumSoftCapUsd: new BN(aumSoftCapUsd),
          lpTokenName,
          lpTokenSymbol,
          lpTokenUri
        });
      }
    );

  program
    .command("add-pool-part-two")
    .description("Create a new pool (part two)")
    .argument("<string>", "Pool name")
    .requiredOption(
      "--genesis-lock-campaign-duration <bigint>",
      "Genesis lock campaign duration in seconds"
    )
    .requiredOption(
      "--genesis-reserved-grant-duration <bigint>",
      "Genesis reserved grant duration in seconds"
    )
    .requiredOption(
      "--genesis-lock-campaign-start-date <bigint>",
      "Genesis lock campaign timestamp in seconds"
    )
    .action(async (poolName, options) => {
      await client.addPoolPartTwo({
        name: poolName,
        genesisLockCampaignDuration: new BN(options.genesisLockCampaignDuration),
        genesisReservedGrantDuration: new BN(options.genesisReservedGrantDuration),
        genesisLockCampaignStartDate: new BN(options.genesisLockCampaignStartDate)
      });
    });

  program
    .command("get-pool")
    .description("Print metadata for the pool")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      client.prettyPrint(await client.getPool(poolName));
    });

  program
    .command("get-pools")
    .description("Print metadata for all pools")
    .action(async () => {
      client.prettyPrint(await client.getPools());
    });

  program
    .command("remove-pool")
    .description("Remove the pool")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      return client.removePool(poolName);
    });

  program
    .command("add-custody")
    .description("Add a new token custody to the pool")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Oracle")
    .argument("<string>", "Trade Oracle")
    .option("-s, --stablecoin", "Stablecoin custody")
    .option("-v, --virtual", "Virtual asset custody")
    .option("--allow-swap <boolean>", "Allow swap")
    .option("--allow-trade <boolean>", "Allow trade")
    .requiredOption(
      "-m, --max-cumulative-short-position-size-usd <int>",
      "Max cumulative short position size usd"
    )
    .action(
      async (poolName, tkMint, oracle, tradeOracle, options) => {
        const tokenMint = new PublicKey(tkMint);
        const isStable = options.stablecoin ?? false;
        const allowSwap = options.allowSwap ?? true;
        const allowTrade = options.allowTrade ?? true;
        const maxCumulativeShortPositionSizeUsd = new BN(options.maxCumulativeShortPositionSizeUsd);

        const pricingConfig: PricingParams = {
          maxInitialLeverage: 1_200_000, // x120
          maxLeverage: 3_000_000, // x300
          maxPositionLockedUsd: new BN(250_000_000_000), // $250k
          maxCumulativeShortPositionSizeUsd: maxCumulativeShortPositionSizeUsd.mul(
            new BN(10 ** 6)
          ),
        };
      
        const fees: Fees = {
          swapIn: 10,
          swapOut: 10,
          stableSwapIn: 10,
          stableSwapOut: 10,
          addLiquidity: 10,
          removeLiquidity: 10,
          closePosition: 16,
          liquidation: 16,
          feeMax: 200,
          padding: [0, 0],
          padding2: new BN(0),
        };
      
        const borrowRate: BorrowRateParams = {
          maxHourlyBorrowInterestRate: new BN(80_000), // 0.008%
        };
      
        const pool = await client.getPool(poolName);
      
        const spot = pool.ratios.findIndex((r) => r.target === 0);
      
        if (spot == -1) {
          throw new Error("Does not have a spot");
        }
      
        pool.ratios[spot] = {
          target: 5000, // 50%
          min: 10, // 0.1%
          max: 10_000, // 100%
          padding: [0, 0],
        };
      
        const ratios = client.adjustTokenRatios(pool.ratios);
      
        await client.addCustody({
          poolName,
          tokenMint,
          isStable,
          oracle,
          tradeOracle,
          pricingConfig,
          fees,
          borrowRate,
          ratios,
          allowSwap,
          allowTrade
        });
      }
    );

  program
    .command("get-custody")
    .description("Print metadata for the token custody")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      client.prettyPrint(await client.getCustody(poolName, new PublicKey(tokenMint)));
    });

  program
    .command("get-asset-prices")
    .description("Print current asset prices")
    .action(async () => {
      client.prettyPrint(await client.getAssetPrices());
    });

  program
    .command("get-custodies")
    .description("Print metadata for all custodies")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      client.prettyPrint(await client.getCustodies(poolName));
    });

  program
    .command("remove-custody")
    .description("Remove the token custody from the pool")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      const pool = await client.getPool(poolName);

      pool.ratios.pop();

      const ratios = client.adjustTokenRatios(pool.ratios);

      await client.removeCustody(poolName, new PublicKey(tokenMint), ratios);
    });

  program
    .command("set-admin")
    .description("Change the program admin **DANGEROUS**")
    .argument("<string>", "New admin")
    .action(async (newAdmin) => {
      const txId = await client.setAdmin(new PublicKey(newAdmin));

      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("set-pool-liquidity-state")
    .description("Set pool liquidity state to 'active' or 'idle'")
    .argument("<string>", "Pool name")
    .argument("<string>", "State")
    .action(async (poolName, state) => {
      const s = {
        idle: 1,
        active: 2,
      }[state as 'idle' | 'active'];
    
      if (typeof s === "undefined") {
        throw new Error("Invalid state");
      }
    
      const txId = await client.setPoolLiquidityState(poolName, s);
    
      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("set-pool-allow-swap")
    .description("Set pool allow swap")
    .argument("<string>", "Pool name")
    .argument("<boolean>", "Allow")
    .action(async (poolName, allow) => {
      const txId = await client.setPoolAllowSwap(poolName, allow);
    
      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("set-pool-allow-trade")
    .description("Set pool allow trade")
    .argument("<string>", "Pool name")
    .argument("<boolean>", "Allow")
    .action(async (poolName, allow) => {
      const txId = await client.setPoolAllowTrade(poolName, allow);
    
      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("set-pool-aum-soft-cap-usd")
    .description("Set pool AUM soft cap in USD")
    .argument("<string>", "Pool name")
    .argument("<number>", "aum soft cap usd")
    .action(async (poolName, aumSoftCapUsd) => {
      const txId = await client.setPoolAumSoftCapUsd(poolName, new BN(aumSoftCapUsd * 10 ** 6));

      console.log(`Transaction succeeded: ${txId}`);
    });

  program
    .command("add-liquidity")
    .description("Deposit liquidity to the custody")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .requiredOption("-i, --amount-in <int>", "Amount to deposit")
    .requiredOption(
      "-o, --min-amount-out <int>",
      "Minimum LP amount to receive"
    )
    .action(async (poolName, tokenMint, options) => {
      await client.addLiquidity({
        poolName,
        tokenMint: new PublicKey(tokenMint),
        amountIn: new BN(options.amountIn),
        minLpAmountOut: new BN(options.minAmountOut),
        oraclePrices: await client.getAssetPrices(),
      });
    });

  program
    .command("long")
    .description("Open a long position with swap")
    .argument("<string>", "Pool name")
    .argument("<mint>", "Principal token mint (e.g., BTC)")
    .argument("<collateralMint>", "Collateral token mint (e.g., BONK)")
    .requiredOption("-c, --collateral <int>", "Collateral amount")
    .requiredOption("-p, --price <int>", "Oracle mid-price")
    .requiredOption("-l, --leverage <int>", "Leverage")
    .action(async (poolName, mint, collateralMint, options) => {
      console.log(await client.openPositionWithSwapLong({
        poolName,
        mint: new PublicKey(mint),
        collateralMint: new PublicKey(collateralMint),
        collateralAmount: new BN(options.collateral),
        price: new BN(options.price),
        leverage: parseInt(options.leverage),
        oraclePrices: await client.getAssetPrices(),
      }));
    });

  program
    .command("close-long")
    .description("Close a long position")
    .argument("<positionKey>", "Position public key")
    .requiredOption("-p, --price <int>", "Oracle mid-price")
    .action(async (positionKey, options) => {
      const oraclePrices = await client.getAssetPrices();
  
      const txSig = await client.closePositionLong({
        positionKey: new PublicKey(positionKey),
        price: new BN(options.price),
        oraclePrices,
      });
  
      console.log("Transaction Signature:", txSig);
    });

  program
    .command("get-user-position")
    .description("Print user position metadata")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (wallet, poolName, tokenMint, side) => {
      client.prettyPrint(
        await client.getUserPosition(
          new PublicKey(wallet),
          poolName,
          new PublicKey(tokenMint),
          side
        )
      );
    });

  program
    .command("get-user-positions")
    .description("Print all user positions")
    .argument("<pubkey>", "User wallet")
    .action(async (wallet) => {
      client.prettyPrint(await client.getUserPositions(new PublicKey(wallet)));
    });

  program
    .command("get-all-positions")
    .description("Print all open positions")
    .action(async () => {
      client.prettyPrint(await client.getAllPositions());
    });

  program
    .command("get-add-liquidity-amount-and-fee")
    .description("Compute LP amount returned and fee for add liquidity")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .requiredOption("-a, --amount <bigint>", "Token amount")
    .action(async (poolName, tokenMint, options) => {
      client.prettyPrint(
        await client.getAddLiquidityAmountAndFee({
          poolName,
          tokenMint: new PublicKey(tokenMint),
          amount: new BN(options.amount),
          oraclePrices: await client.getAssetPrices(),
        })
      );
    });

  program
    .command("get-remove-liquidity-amount-and-fee")
    .description("Compute token amount returned and fee for remove liquidity")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .requiredOption("-a, --amount <bigint>", "LP token amount")
    .action(async (poolName, tokenMint, options) => {
      client.prettyPrint(
        await client.getRemoveLiquidityAmountAndFee({
          poolName,
          tokenMint: new PublicKey(tokenMint),
          lpAmount: new BN(options.amount),
          oraclePrices: await client.getAssetPrices(),
        }),
      );
    });

  program
    .command("get-entry-price-and-fee")
    .description("Compute price and fee to open a position")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<pubkey>", "Collateral mint")
    .argument("<string>", "Position side (long / short)")
    .requiredOption("-c, --collateral <bigint>", "Collateral")
    .requiredOption(
      "-s, --leverage <bigint>",
      "Leverage (10000 = x1, 500000 = x50)"
    )
    .action(async (poolName, tokenMint, collateralMint, side, options) => {
      client.prettyPrint(
        await client.getEntryPriceAndFee({
          poolName,
          tokenMint: new PublicKey(tokenMint),
          collateralMint: new PublicKey(collateralMint),
          collateral: new BN(options.collateral),
          leverage: options.leverage,
          side,
          oraclePrices: await client.getAssetPrices(),
        })
      );
    });

  program
    .command("get-lp-token-mint")
    .description("Get LP token mint address for the pool")
    .argument("<string>", "Pool name")
    .action((poolName) => {
      client.prettyPrint(client.getPoolLpTokenKey(poolName));
    });

  program
    .command("get-liquidation-price")
    .description("Compute liquidation price for the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .option("-a, --add-collateral <bigint>", "Collateral to add")
    .option("-r, --remove-collateral <bigint>", "Collateral to remove")
    .action(async (w, poolName, tkMint, side, options) => {
      const wallet = new PublicKey(w);
      const tokenMint = new PublicKey(tkMint);
      const addCollateral = new BN(options.addCollateral);
      const removeCollateral = new BN(options.removeCollateral);

      await client.getLiquidationPrice({
        wallet,
        poolName,
        tokenMint,
        collateralMint: await client.getCollateralCustodyMint(wallet, poolName, tokenMint, side),
        side,
        addCollateral,
        removeCollateral,
        oraclePrices: await client.getAssetPrices(),
      });
    });

  program
    .command("get-liquidation-state")
    .description("Get liquidation state of the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (w, poolName, tkMint, side) => {
      const wallet = new PublicKey(w);
      const tokenMint = new PublicKey(tkMint);

      await client.getLiquidationState({
        wallet,
        poolName,
        tokenMint,
        collateralMint: await client.getCollateralCustodyMint(wallet, poolName, tokenMint, side),
        side,
        oraclePrices: await client.getAssetPrices(),
      });
    });

  program
    .command("get-pnl")
    .description("Compute PnL of the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (w, poolName, tkMint, side) => {
      const wallet = new PublicKey(w);
      const tokenMint = new PublicKey(tkMint);

      await client.getPnl({
        wallet,
        poolName,
        tokenMint,
        collateralMint: await client.getCollateralCustodyMint(wallet, poolName, tokenMint, side),
        side,
        oraclePrices: await client.getAssetPrices(),
      })
    });

  program
    .command("get-swap-amount-and-fees")
    .description("Compute amount out and fees for the swap")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint in")
    .argument("<pubkey>", "Token mint out")
    .requiredOption("-i, --amount-in <bigint>", "Token amount to be swapped")
    .action(async (poolName, tokenMintIn, tokenMintOut, options) => {
      client.prettyPrint(
        await client.getSwapAmountAndFees({
          poolName,
          tokenMintIn: new PublicKey(tokenMintIn),
          tokenMintOut: new PublicKey(tokenMintOut),
          amountIn: new BN(options.amountIn),
          oraclePrices: await client.getAssetPrices(),
        })
      );
    });

  program
    .command("get-aum")
    .description("Get assets under management")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      client.prettyPrint(await client.getAum(poolName));
    });

  program
    .command("set-custodies-ratios")
    .description(
      "Change custody ratios. Specify ratios for all custodies of the pool. Ratios in BPS, 10000 = 100%, 50 = 0.5%"
    )
    .argument("<string>", "Pool name")
    .argument("<numbers...>", "Custody Ratio (min target max)")
    .action(async (poolName, rawRatios) => {
      console.log("poolName", poolName);

      const ratios: {
        min: number;
        target: number;
        max: number;
        padding: [0, 0];
      }[] = [];

      if (rawRatios.length === 0) {
        throw new Error("Ratios cannot be empty");
      }

      if (rawRatios.length % 3 !== 0) {
        throw new Error("Missing ratio");
      }

      for (let i = 0; i < rawRatios.length; i += 3) {
        ratios.push({
          min: rawRatios[i],
          target: rawRatios[i + 1],
          max: rawRatios[i + 2],
          padding: [0, 0],
        });
      }

      // Add ratios up to 10
      for (let i = ratios.length; i < 10; i++) {
        ratios.push({
          min: 0,
          target: 0,
          max: 0,
          padding: [0, 0],
        });
      }

      const txId = await client.setCustodiesRatio(poolName, ratios);

      console.log(`Transaction succeeded: ${txId}`);
    });

  await program.parseAsync(process.argv);

  if (!process.argv.slice(2).length) {
    program.outputHelp();
  }
})();
