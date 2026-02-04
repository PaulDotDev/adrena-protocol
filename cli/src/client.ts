import {
  setProvider,
  Program,
  AnchorProvider,
  workspace,
  utils,
  BN,
} from "@coral-xyz/anchor";
import { Adrena } from "../../target/types/adrena";
import {
  PublicKey,
  SystemProgram,
  Keypair,
  SYSVAR_RENT_PUBKEY,
  AccountMeta,
  ComputeBudgetProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import fetch from "node-fetch";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createAssociatedTokenAccountInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { sha256 } from "@noble/hashes/sha256";
import encode from "bs58";
import { readFileSync } from "fs";
import {
  TokenRatio,
  PositionSide,
  PricingParams,
  Fees,
  BorrowRateParams,
  AmountAndFee,
  NewPositionPricesAndFee,
  ProfitAndLoss,
  SwapAmountAndFees,
  Custody,
  InitOneParams,
  CustodyExtended,
  LimitedString,
  ChaosLabsBatchPrices,
  OraclePricesSetup,
} from "./types";
import {
  GovernanceConfig,
  GoverningTokenConfigAccountArgs,
  GoverningTokenType,
  MintMaxVoteWeightSource,
  MintMaxVoteWeightSourceType,
  SetRealmAuthorityAction,
  VoteThresholdType,
  VoteTipping,
  getGoverningTokenHoldingAddress,
  getRealmConfigAddress,
  getTokenOwnerRecordAddress,
  withCreateGovernance,
  withCreateNativeTreasury,
  withCreateRealm,
  withSetRealmAuthority,
} from "@solana/spl-governance";

const governanceProgram = new PublicKey(
  "GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw"
);

const mplTokenMetadataProgram = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

export class AdrenaClient {
  provider: AnchorProvider;
  program: Program<Adrena>;
  admin: Keypair;

  // PDAs
  authority: { publicKey: PublicKey; bump: number };
  lmTokenMint: { publicKey: PublicKey; bump: number };
  lmStaking: { publicKey: PublicKey; bump: number };
  oracle: { publicKey: PublicKey; bump: number };
  cortex: { publicKey: PublicKey; bump: number };
  governanceTokenMint: { publicKey: PublicKey; bump: number };
  lmStakingStakedTokenVault: { publicKey: PublicKey; bump: number };
  lmStakingRewardTokenVault: { publicKey: PublicKey; bump: number };
  lmStakingLmRewardTokenVault: { publicKey: PublicKey; bump: number };
  vestRegistry: { publicKey: PublicKey; bump: number };
  lmTokenMintMetadata: PublicKey;

  constructor(clusterUrl: string, adminKey: string) {
    console.log("clusterUrl", clusterUrl);

    this.provider = AnchorProvider.local(clusterUrl, {
      commitment: "processed",
      preflightCommitment: "processed",
    });

    setProvider(this.provider);

    this.program = workspace.Adrena as Program<Adrena>;
    this.admin = Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(readFileSync(adminKey).toString()))
    );

    this.authority = this.findProgramAddress("transfer_authority");
    this.lmTokenMint = this.findProgramAddress("lm_token_mint");

    this.lmTokenMintMetadata = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        mplTokenMetadataProgram.toBuffer(),
        this.lmTokenMint.publicKey.toBuffer(),
      ],
      mplTokenMetadataProgram
    )[0];

    this.lmStaking = this.findProgramAddress("staking", [
      this.lmTokenMint.publicKey,
    ]);
    this.cortex = this.findProgramAddress("cortex");
    this.oracle = this.findProgramAddress("oracle");
    this.governanceTokenMint = this.findProgramAddress("governance_token_mint");
    this.lmStakingStakedTokenVault = this.findProgramAddress(
      "staking_staked_token_vault",
      [this.lmStaking.publicKey]
    );
    this.lmStakingRewardTokenVault = this.findProgramAddress(
      "staking_reward_token_vault",
      [this.lmStaking.publicKey]
    );
    this.lmStakingLmRewardTokenVault = this.findProgramAddress(
      "staking_lm_reward_token_vault",
      [this.lmStaking.publicKey]
    );
    this.vestRegistry = this.findProgramAddress("vest_registry");

    BN.prototype.toJSON = function () {
      return this.toString(10);
    };
  }

  getVestPda(owner: PublicKey): {
    publicKey: PublicKey;
    bump: number;
  } {
    return this.findProgramAddress("vest", [owner]);
  }

  getGenesisLockAddress(pool: PublicKey): {
    publicKey: PublicKey;
    bump: number;
  } {
    return this.findProgramAddress("genesis_lock", [pool]);
  }

  getStakingThreadAuthorityPda(stakingPda: PublicKey): {
    publicKey: PublicKey;
    bump: number;
  } {
    return this.findProgramAddress("staking-thread-authority", [stakingPda]);
  }

  findProgramAddress(
    label: string,
    extraSeeds: (string | PublicKey | number[])[] | null = null
  ): {
    publicKey: PublicKey;
    bump: number;
  } {
    const seeds = [Buffer.from(utils.bytes.utf8.encode(label))];

    if (extraSeeds) {
      for (const extraSeed of extraSeeds) {
        if (typeof extraSeed === "string") {
          seeds.push(Buffer.from(utils.bytes.utf8.encode(extraSeed)));
        } else if (Array.isArray(extraSeed)) {
          seeds.push(Buffer.from(extraSeed));
        } else if (extraSeed instanceof PublicKey) {
          seeds.push(extraSeed.toBuffer());
        }
      }
    }

    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      seeds,
      this.program.programId
    );

    return { publicKey, bump };
  }

  adjustTokenRatios(ratios: TokenRatio[]): TokenRatio[] {
    const len = ratios.reduce(
      (acc, ratio) => acc + (ratio.target != 0 ? 1 : 0),
      0
    );

    if (len == 0) {
      return ratios;
    }

    const balancedRatios = ratios.map((ratio) => {
      if (ratio.target == 0) {
        return ratio;
      }

      return {
        ...ratio,
        target: Math.floor(10_000 / len),
      };
    });

    if (10_000 % len !== 0) {
      const e = balancedRatios.find((x) => x.target > 0);

      if (e)
        e.target += 10_000 % len;
    }

    return balancedRatios;
  }

  getCortex() {
    return this.program.account.cortex.fetch(this.cortex.publicKey);
  }

  getPoolKey(name: string): PublicKey {
    return this.findProgramAddress("pool", [name]).publicKey;
  }

  getLpStaking(lpTokenMintPda: PublicKey): PublicKey {
    return this.findProgramAddress("staking", [lpTokenMintPda]).publicKey;
  }

  getLmStaking(lmTokenMintPda: PublicKey): PublicKey {
    return this.findProgramAddress("staking", [lmTokenMintPda]).publicKey;
  }

  getLpStakingRewardTokenVault(lpStakingPda: PublicKey): PublicKey {
    return this.findProgramAddress("staking_reward_token_vault", [lpStakingPda])
      .publicKey;
  }

  getPool(name: string) {
    const poolKey = this.getPoolKey(name);
    return this.program.account.pool.fetch(poolKey);
  }

  async getPools() {
    const cortex = await this.getCortex();

    return this.program.account.pool.fetchMultiple(
      cortex.pools.filter(
        (pool) => !pool.equals(PublicKey.default)
      ) as PublicKey[]
    );
  }

  getPoolLpTokenKey(name: string): PublicKey {
    return this.findProgramAddress("lp_token_mint", [this.getPoolKey(name)])
      .publicKey;
  }

  getCustodyKey(poolName: string, tokenMint: PublicKey): PublicKey {
    return this.findProgramAddress("custody", [
      this.getPoolKey(poolName),
      tokenMint,
    ]).publicKey;
  }

  getDaoRealmKey(realmName: string): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("governance"), Buffer.from(realmName)],
      governanceProgram
    )[0];
  }

  getGovernanceTokenKey(): PublicKey {
    return this.findProgramAddress("governance_token_mint").publicKey;
  }

  getCustodyTokenAccountKey(poolName: string, tokenMint: PublicKey): PublicKey {
    return this.findProgramAddress("custody_token_account", [
      this.getPoolKey(poolName),
      tokenMint,
    ]).publicKey;
  }

  async getCustodyOracleKey(
    poolName: string,
    tokenMint: PublicKey
  ): Promise<LimitedString> {
    return (await this.getCustody(poolName, tokenMint)).oracle;
  }

  async getCustodyTradeOracleKey(
    poolName: string,
    tokenMint: PublicKey
  ): Promise<LimitedString> {
    return (await this.getCustody(poolName, tokenMint)).tradeOracle;
  }

  getCustody(poolName: string, tokenMint: PublicKey) {
    return this.program.account.custody.fetch(
      this.getCustodyKey(poolName, tokenMint)
    );
  }

  async getCustodies(poolName: string): Promise<CustodyExtended[]> {
    const pool = await this.getPool(poolName);

    const custodiesPda = pool.custodies.filter(
      (x) => !x.equals(PublicKey.default)
    );

    const custodies = (await this.program.account.custody.fetchMultiple(
      custodiesPda
    )) as (Custody | null)[];

    if (custodies.some((custody) => !custody)) {
      throw new Error("Error loading custodies");
    }

    return (custodies.map((c, i) => ({
      ...c,
      pubkey: custodiesPda[i],
    }))) as CustodyExtended[];
  }

  async getCustodyMetas(poolName: string): Promise<AccountMeta[]> {
    const pool = await this.getPool(poolName);
    const custodiesAddresses = pool.custodies.filter(
      (x) => !x.equals(PublicKey.default)
    ) as PublicKey[];

    const custodies = (await this.program.account.custody.fetchMultiple(
      custodiesAddresses
    )) as (Custody | null)[];

    if (custodies.some((custody) => !custody)) {
      throw new Error("Error loading custodies");
    }

    const custodyMetas: AccountMeta[] = [];

    for (const custody of custodiesAddresses) {
      custodyMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: custody,
      });
    }

    return custodyMetas;
  }

  async getCollateralCustodyMint(
    wallet: PublicKey,
    poolName: string,
    tokenMint: PublicKey,
    side: PositionSide
  ): Promise<PublicKey> {
    const custodyAccount = (
      await this.getUserPosition(wallet, poolName, tokenMint, side)
    ).collateralCustody;

    return (await this.program.account.custody.fetch(custodyAccount)).mint;
  }

  getPositionKey(
    wallet: PublicKey,
    poolName: string,
    tokenMint: PublicKey,
    side: PositionSide
  ): PublicKey {
    const pool = this.getPoolKey(poolName);
    const custody = this.getCustodyKey(poolName, tokenMint);

    return this.findProgramAddress("position", [
      wallet,
      pool,
      custody,
      side === "long" ? [1] : [2],
    ]).publicKey;
  }

  async getUserPosition(
    wallet: PublicKey,
    poolName: string,
    tokenMint: PublicKey,
    side: PositionSide
  ) {
    return this.program.account.position.fetch(
      this.getPositionKey(wallet, poolName, tokenMint, side)
    );
  }

  async getUserPositions(wallet: PublicKey) {
    const data = encode.encode(
      Buffer.concat([
        this.getAccountDiscriminator("Position"),
        wallet.toBuffer(),
      ])
    );

    const positions = await this.provider.connection.getProgramAccounts(
      this.program.programId,
      {
        filters: [{ dataSize: 232 }, { memcmp: { bytes: data, offset: 0 } }],
      }
    );

    return Promise.all(
      positions.map((position) => {
        return this.program.account.position.fetch(position.pubkey);
      })
    );
  }

  async getCustodyPositionList(
    pool: PublicKey,
    custody: PublicKey
  ): Promise<
    {
      pubkey: PublicKey;
      account: {
        side: PositionSide;
        owner: PublicKey;
        custody: PublicKey;
        collateralCustody: PublicKey;
      };
    }[]
  > {
    const data = encode.encode(Buffer.concat([pool.toBuffer(), custody.toBuffer()]));

    const ret = await this.provider.connection.getProgramAccounts(
      this.program.programId,
      {
        filters: [{ dataSize: 288 }, { memcmp: { bytes: data, offset: 48 } }],
        dataSlice: {
          // Target the following information about the position:
          // side
          // owner
          // custody
          // collateralCustody
          offset: 9,
          length: 1 + 1 + 1 + 4 + 32 + 32 + 32 + 32,
        },
      }
    );

    return ret.map((x) => ({
      pubkey: x.pubkey,
      account: {
        side: x.account.data[0] === 1 ? ("long" as const) : ("short" as const),
        owner: new PublicKey(x.account.data.slice(7, 39)),
        custody: new PublicKey(x.account.data.slice(71, 103)),
        collateralCustody: new PublicKey(x.account.data.slice(103, 135)),
      },
    }));
  }

  getAllPositions() {
    return this.program.account.position.all();
  }

  getAccountDiscriminator(name: string): Buffer {
    return Buffer.from(sha256(`account:${name}`).slice(0, 8));
  }

  getMethodDiscriminator(name: string): Buffer {
    return Buffer.from(sha256(`global:${name}`).slice(0, 8));
  }

  getTime(): number {
    const now = new Date();
    const utcMillisecondsSinceEpoch =
      now.getTime() + now.getTimezoneOffset() * 60 * 1_000;

    return utcMillisecondsSinceEpoch / 1_000;
  }

  log = (...messages: string[]): void => {
    const date = new Date();
    const dateStr = date.toDateString();
    const time = date.toLocaleTimeString();

    console.log(`[${dateStr} ${time}] ${messages.join(", ")}`);
  };

  prettyPrint(v: any): void {
    console.log(JSON.stringify(v, null, 2));
  }

  ///////
  // instructions

  async createDaoGovernance(
    realmName: string,
    minCommunityTokensToCreateProposal: BN
  ): Promise<PublicKey> {
    // Use the Admin as the authority
    const authority = this.provider.wallet.publicKey;
    const payer = authority;

    const programVersion = 3;

    const realmPubkey = this.getDaoRealmKey(realmName);

    const communityMint = this.getGovernanceTokenKey();

    const instructions: TransactionInstruction[] = [];

    const tokenOwnerRecordAddress = await getTokenOwnerRecordAddress(
      governanceProgram,
      realmPubkey,
      communityMint,
      authority
    );

    const governancePubkey = await withCreateGovernance(
      instructions,
      governanceProgram,
      programVersion,
      realmPubkey,
      undefined, // GovernedAccount
      new GovernanceConfig({
        minCommunityTokensToCreateProposal,
        minCouncilTokensToCreateProposal: new BN(0), // unused
        communityVoteThreshold: {
          type: VoteThresholdType.YesVotePercentage,
          value: 45,
        },
        minInstructionHoldUpTime: 0,
        baseVotingTime: 3 * (3600 * 24), // 3 days
        communityVoteTipping: VoteTipping.Strict,
        councilVoteThreshold: {
          type: VoteThresholdType.Disabled,
          value: undefined,
        },
        councilVetoVoteThreshold: {
          type: VoteThresholdType.Disabled,
          value: undefined,
        },
        communityVetoVoteThreshold: {
          type: VoteThresholdType.Disabled,
          value: undefined,
        },
        councilVoteTipping: VoteTipping.Disabled,
        votingCoolOffTime: 0, // Proposal can be executed as soon as it passes
        depositExemptProposalCount: 0,
      }),
      tokenOwnerRecordAddress,
      payer,
      authority,
      undefined // voterWeightRecord
    );

    const tx = new Transaction();

    tx.add(...instructions);
    tx.recentBlockhash = (
      await this.provider.connection.getLatestBlockhash()
    ).blockhash;
    tx.feePayer = payer;

    const signedTransaction = await this.provider.wallet.signTransaction(tx);

    const txId = await this.provider.connection.sendRawTransaction(
      signedTransaction.serialize()
    );

    const confirmationStatus =
      await this.provider.connection.confirmTransaction(txId, "confirmed");

    if (confirmationStatus.value.err) {
      console.error(`Transaction failed: ${confirmationStatus.value.err}`);
    } else {
      console.log(`Transaction succeeded: ${txId}`);
    }

    return governancePubkey;
  }

  async createDaoNativeTreasury(
    governancePubkey: PublicKey
  ): Promise<PublicKey> {
    // Use the Admin as the authority
    const authority = this.provider.wallet.publicKey;
    const payer = authority;

    const programVersion = 3;

    const instructions: TransactionInstruction[] = [];

    const nativeTreasuryPubkey = await withCreateNativeTreasury(
      instructions,
      governanceProgram,
      programVersion,
      governancePubkey,
      payer
    );

    const tx = new Transaction();

    tx.add(...instructions);
    tx.recentBlockhash = (
      await this.provider.connection.getLatestBlockhash()
    ).blockhash;
    tx.feePayer = payer;

    const signedTransaction = await this.provider.wallet.signTransaction(tx);

    const txId = await this.provider.connection.sendRawTransaction(
      signedTransaction.serialize()
    );

    const confirmationStatus =
      await this.provider.connection.confirmTransaction(txId, "confirmed");

    if (confirmationStatus.value.err) {
      console.error(`Transaction failed: ${confirmationStatus.value.err}`);
    } else {
      console.log(`Transaction succeeded: ${txId}`);
    }

    return nativeTreasuryPubkey;
  }

  async createDaoRealm(
    name: string,
    minCommunityWeightToCreateGovernance: BN
  ): Promise<PublicKey> {
    // Use the Admin as the authority
    const realmAuthority = this.provider.wallet.publicKey;
    const payer = realmAuthority;

    const instructions: TransactionInstruction[] = [];

    // Force program version
    const programVersion = 3;

    const communityMint = this.getGovernanceTokenKey();

    console.log("Community Mint:", communityMint.toBase58());

    const communityMintMaxVoteWeightSource = new MintMaxVoteWeightSource({
      /// Fraction (10^10 precision) of the governing mint supply is used as max vote weight
      /// The default is 100% (10^10) to use all available mint supply for voting
      type: MintMaxVoteWeightSourceType.SupplyFraction,

      // 100%
      value: new BN(10 ** 10),
    });

    const communityTokenConfig: GoverningTokenConfigAccountArgs =
      new GoverningTokenConfigAccountArgs({
        tokenType: GoverningTokenType.Membership,
        voterWeightAddin: undefined,
        maxVoterWeightAddin: undefined,
      });

    const realmPubkey = await withCreateRealm(
      instructions,
      governanceProgram,
      programVersion,
      name,
      realmAuthority,
      communityMint,
      payer,
      undefined /* council mint */,
      communityMintMaxVoteWeightSource,
      // Governance token mint is 6 decimals
      new BN(10 ** 6).mul(minCommunityWeightToCreateGovernance),
      communityTokenConfig,
      undefined /* councilTokenConfig */
    );

    const tx = new Transaction();

    tx.add(...instructions);
    tx.recentBlockhash = (
      await this.provider.connection.getLatestBlockhash()
    ).blockhash;
    tx.feePayer = payer;

    const signedTransaction = await this.provider.wallet.signTransaction(tx);

    const txId = await this.provider.connection.sendRawTransaction(
      signedTransaction.serialize()
    );

    const confirmationStatus =
      await this.provider.connection.confirmTransaction(txId, "confirmed");

    if (confirmationStatus.value.err) {
      console.error(`Transaction failed: ${confirmationStatus.value.err}`);
    } else {
      console.log(`Transaction succeeded: ${txId}`);
    }

    return realmPubkey;
  }

  async setRealmAuthority(
    realmPubkey: PublicKey,
    newRealmAuthority: PublicKey
  ): Promise<void> {
    // The default authority
    const realmAuthority = this.provider.wallet.publicKey;
    const payer = realmAuthority;

    const instructions: TransactionInstruction[] = [];

    // Force program version
    const programVersion = 3;

    console.log("Public key realmPubkey", realmPubkey.toBase58());
    console.log("Public key newRealmAuthority:", newRealmAuthority.toBase58());

    withSetRealmAuthority(
      instructions,
      governanceProgram,
      programVersion,
      realmPubkey,
      realmAuthority,
      newRealmAuthority,
      SetRealmAuthorityAction.SetUnchecked
    );

    const tx = new Transaction();

    tx.add(...instructions);
    tx.recentBlockhash = (
      await this.provider.connection.getLatestBlockhash()
    ).blockhash;
    tx.feePayer = payer;

    const signedTransaction = await this.provider.wallet.signTransaction(tx);

    const txId = await this.provider.connection.sendRawTransaction(
      signedTransaction.serialize()
    );

    const confirmationStatus =
      await this.provider.connection.confirmTransaction(txId, "confirmed");

    if (confirmationStatus.value.err) {
      console.error(`Transaction failed: ${confirmationStatus.value.err}`);
    } else {
      console.log(`Transaction succeeded: ${txId}`);
    }
  }

  async claimVest(): Promise<string> {
    const lmTokenAccount = await getAssociatedTokenAddress(
      this.lmTokenMint.publicKey,
      this.provider.wallet.publicKey
    );

    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const preInstructions: TransactionInstruction[] = [];

    // Create LM ATA if doesn't exist
    if (!(await this.provider.connection.getAccountInfo(lmTokenAccount))) {
      preInstructions.push(
        createAssociatedTokenAccountInstruction(
          this.provider.wallet.publicKey,
          lmTokenAccount,
          this.provider.wallet.publicKey,
          this.lmTokenMint.publicKey
        )
      );
    }

    return this.program.methods
      .claimVest()
      .accountsStrict({
        owner: this.provider.wallet.publicKey,
        receivingAccount: lmTokenAccount,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        vest: this.getVestPda(this.provider.wallet.publicKey).publicKey,
        vestRegistry: this.vestRegistry.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        governanceTokenMint: this.governanceTokenMint.publicKey,
        governanceRealm: cortexAccount.governanceRealm,
        governanceRealmConfig: await getRealmConfigAddress(
          governanceProgram,
          cortexAccount.governanceRealm
        ),
        governanceGoverningTokenHolding: await getGoverningTokenHoldingAddress(
          governanceProgram,
          cortexAccount.governanceRealm,
          this.governanceTokenMint.publicKey
        ),
        governanceGoverningTokenOwnerRecord: await getTokenOwnerRecordAddress(
          governanceProgram,
          cortexAccount.governanceRealm,
          this.governanceTokenMint.publicKey,
          this.provider.wallet.publicKey
        ),
        governanceProgram,
        adrenaProgram: this.program.programId,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        payer: this.provider.wallet.publicKey,
        caller: this.provider.wallet.publicKey,
      })
      .preInstructions(preInstructions)
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initOneCore(
    feeRedistributionMint: PublicKey,
    protocolFeeRecipient: PublicKey,
    config: InitOneParams
  ): Promise<void> {
    await this.program.methods
      .initOneCore(config)
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        feeRedistributionMint,
        protocolFeeRecipient,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initOracle(oraclePrices: OraclePricesSetup[]): Promise<void> {
    await this.program.methods
    .initOracle({
      oraclePrices,
    })
    .accountsStrict({
      admin: this.admin.publicKey,
      payer: this.provider.wallet.publicKey,
      cortex: this.cortex.publicKey,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      oracle: this.oracle.publicKey,
    })
    .rpc()
    .catch((err) => {
      console.error(err);
      throw err;
    });
  }

  async initTwoLmTokenMetadata(): Promise<void> {
    await this.program.methods
      .initTwoLmTokenMetadata()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        lmTokenMintMetadata: this.lmTokenMintMetadata,
        systemProgram: SystemProgram.programId,
        mplTokenMetadataProgram,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  disableTokensFreezeCapabilities(poolName: string): Promise<string> {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    return this.program.methods
      .disableTokensFreezeCapabilities()
      .accountsStrict({
        admin: this.admin.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        lpTokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        pool: this.getPoolKey(poolName),
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initThreeGovernance(governanceRealm: PublicKey): Promise<void> {
    await this.program.methods
      .initThreeGovernance()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        governanceRealm,
        governanceProgram,
        governanceTokenMint: this.governanceTokenMint.publicKey,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async InitFourVesting(): Promise<void> {
    await this.program.methods
      .initFourVesting()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        vestRegistry: this.vestRegistry.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLmStakingOne() {
    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingStakedTokenVault } = this.findProgramAddress(
      "staking_staked_token_vault",
      [this.lmStaking.publicKey]
    );
    const { publicKey: stakingRewardTokenVault } = this.findProgramAddress(
      "staking_reward_token_vault",
      [this.lmStaking.publicKey]
    );

    await this.program.methods
      .initStakingOne({
        stakingType: 1, // LM = 1
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: this.lmStaking.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        feeRedistributionMint,
        adrenaProgram: this.program.programId,
        stakingStakedTokenVault,
        stakingStakedTokenMint: this.lmTokenMint.publicKey,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLmStakingTwo() {
    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingRewardTokenVault } = this.findProgramAddress(
      "staking_reward_token_vault",
      [this.lmStaking.publicKey]
    );

    await this.program.methods
      .initStakingTwo()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: this.lmStaking.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        // rent: SYSVAR_RENT_PUBKEY,
        feeRedistributionMint,
        // adrenaProgram: this.program.programId,
        stakingRewardTokenVault,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLmStakingThree() {
    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    await this.program.methods
      .initStakingThree()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: this.lmStaking.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        feeRedistributionMint,
        stakingLmRewardTokenVault: this.lmStakingLmRewardTokenVault.publicKey,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLmStakingFour() {
    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingStakedTokenVault } = this.findProgramAddress(
      "staking_staked_token_vault",
      [this.lmStaking.publicKey]
    );

    const { publicKey: stakingRewardTokenVault } = this.findProgramAddress(
      "staking_reward_token_vault",
      [this.lmStaking.publicKey]
    );

    await this.program.methods
      .initStakingFour()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: this.lmStaking.publicKey,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        feeRedistributionMint,
        adrenaProgram: this.program.programId,
        stakingRewardTokenVault,
        stakingStakedTokenMint: this.lmTokenMint.publicKey,
        stakingLmRewardTokenVault: this.lmStakingLmRewardTokenVault.publicKey,
        stakingStakedTokenVault,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLpStakingOne(poolName: string) {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    const { publicKey: lpStaking } = this.findProgramAddress("staking", [
      lpTokenMint,
    ]);

    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingStakedTokenVault } = this.findProgramAddress(
      "staking_staked_token_vault",
      [lpStaking]
    );

    await this.program.methods
      .initStakingOne({
        stakingType: 2, // LP = 2
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: lpStaking,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        feeRedistributionMint,
        adrenaProgram: this.program.programId,
        stakingStakedTokenVault,
        stakingStakedTokenMint: lpTokenMint,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLpStakingTwo(poolName: string) {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    const { publicKey: lpStaking } = this.findProgramAddress("staking", [
      lpTokenMint,
    ]);

    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingRewardTokenVault } = this.findProgramAddress(
      "staking_reward_token_vault",
      [lpStaking]
    );

    await this.program.methods
      .initStakingTwo()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: lpStaking,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        feeRedistributionMint,
        stakingRewardTokenVault,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLpStakingThree(poolName: string) {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    const { publicKey: lpStaking } = this.findProgramAddress("staking", [
      lpTokenMint,
    ]);

    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingLmRewardTokenVault } = this.findProgramAddress(
      "staking_lm_reward_token_vault",
      [lpStaking]
    );

    await this.program.methods
      .initStakingThree()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: lpStaking,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        feeRedistributionMint,
        stakingLmRewardTokenVault,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async initLpStakingFour(poolName: string) {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    const { publicKey: lpStaking } = this.findProgramAddress("staking", [
      lpTokenMint,
    ]);

    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    const feeRedistributionMint: PublicKey =
      cortexAccount.feeRedistributionMint;

    const { publicKey: stakingStakedTokenVault } = this.findProgramAddress(
      "staking_staked_token_vault",
      [lpStaking]
    );

    const { publicKey: stakingRewardTokenVault } = this.findProgramAddress(
      "staking_reward_token_vault",
      [lpStaking]
    );

    const { publicKey: stakingLmRewardTokenVault } = this.findProgramAddress(
      "staking_lm_reward_token_vault",
      [lpStaking]
    );

    await this.program.methods
      .initStakingFour()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        staking: lpStaking,
        cortex: this.cortex.publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        feeRedistributionMint,
        adrenaProgram: this.program.programId,
        stakingRewardTokenVault,
        stakingStakedTokenMint: lpTokenMint,
        stakingLmRewardTokenVault,
        stakingStakedTokenVault,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async addPoolPartOne({
    name,
    aumSoftCapUsd,
    lpTokenName,
    lpTokenSymbol,
    lpTokenUri,
  }: {
    name: string;
    aumSoftCapUsd: BN;
    lpTokenName: string;
    lpTokenSymbol: string;
    lpTokenUri: string;
  }): Promise<void> {
    const pool = this.getPoolKey(name);

    const lpTokenMint = this.getPoolLpTokenKey(name);

    const lpTokenMintMetadata = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        mplTokenMetadataProgram.toBuffer(),
        lpTokenMint.toBuffer(),
      ],
      mplTokenMetadataProgram
    )[0];

    await this.program.methods
      .addPoolPartOne({
        name,
        aumSoftCapUsd,
        lpTokenName,
        lpTokenSymbol,
        lpTokenUri,
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool,
        lpTokenMint,
        lpTokenMintMetadata,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        mplTokenMetadataProgram,
        adrenaProgram: this.program.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async addPoolPartTwo({
    name,
    genesisLockCampaignDuration,
    genesisReservedGrantDuration,
    genesisLockCampaignStartDate,
  }: {
    name: string;
    genesisLockCampaignDuration: BN;
    genesisReservedGrantDuration: BN;
    genesisLockCampaignStartDate: BN;
  }): Promise<void> {
    const pool = this.getPoolKey(name);
    const genesisLock = this.getGenesisLockAddress(pool).publicKey;

    const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
      units: 500_000,
    });
    const preInstructions: TransactionInstruction[] = [modifyComputeUnits];

    await this.program.methods
      .addPoolPartTwo({
        genesisLockCampaignDuration,
        genesisReservedGrantDuration,
        genesisLockCampaignStartDate,
        reservedSpots: {
          none: {},
        },
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool,
        lpTokenMint: this.getPoolLpTokenKey(name),
        genesisLock,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([this.admin])
      .preInstructions(preInstructions)
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  setAdmin(newAdmin: PublicKey): Promise<string> {
    return this.program.methods
      .setAdmin({
        newAdmin,
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        cortex: this.cortex.publicKey,
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async removePool(name: string): Promise<void> {
    await this.program.methods
      .removePool()
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(name),
        systemProgram: SystemProgram.programId,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async loadUserProfilePda(ownerPublicKey: PublicKey) {
    const userProfilePda = PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), ownerPublicKey.toBuffer()],

      this.program.programId
    )[0];

    const p = await this.program.account.userProfile.fetchNullable(
      userProfilePda,
      "processed"
    );

    if (p === null || p.createdAt.isZero()) return null;

    return userProfilePda;
  }

  finalizeGenesisLockCampaign(poolName: string): Promise<string> {
    const poolKey = this.getPoolKey(poolName);
    const genesisLock = this.getGenesisLockAddress(poolKey).publicKey;

    return this.program.methods
      .finalizeGenesisLockCampaign()
      .accountsStrict({
        pool: poolKey,
        cortex: this.cortex.publicKey,
        genesisLock,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        adrenaProgram: this.program.programId,
        caller: this.provider.wallet.publicKey,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async setCustodiesRatio(
    poolName: string,

    // number of ratios must match in order the custodies of the pool and be 100% total
    ratios: {
      min: number;
      target: number;
      max: number;
      padding: [0, 0];
    }[]
  ): Promise<string> {
    const poolKey = this.getPoolKey(poolName);

    const poolAccount = await this.program.account.pool.fetch(poolKey);

    if (!poolAccount.custodies.length) {
      throw new Error("No custodies");
    }

    const custodyKey = poolAccount.custodies[0];

    const custodyAccount = await this.program.account.custody.fetch(custodyKey);

    return this.program.methods
      .setCustodyConfig({
        isStable: custodyAccount.isStable == 0 ? false : true,
        oracle: custodyAccount.oracle,
        tradeOracle: custodyAccount.tradeOracle,
        pricing: custodyAccount.pricing,
        fees: custodyAccount.fees,
        borrowRate: custodyAccount.borrowRate,
        ratios,
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        pool: poolKey,
        cortex: this.cortex.publicKey,
        custody: custodyKey,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  toLimitedStringBuffer(str: string): LimitedString {
    const buffer = new Uint8Array(31);
    const ob = Buffer.from(str, 'utf-8');

    // Recreate a LimitedString
    buffer.set(ob.slice(0, 31), 0);

    return {
      value: Array.from(buffer).map(x => Number(x)),
      length: ob.length,
    };
  }

  async addCustody({
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
    allowTrade,
  }: {
    poolName: string;
    tokenMint: PublicKey;
    isStable: boolean;
    oracle: string;
    tradeOracle: string;
    pricingConfig: PricingParams;
    fees: Fees;
    borrowRate: BorrowRateParams;
    ratios: TokenRatio[];
    allowSwap: boolean;
    allowTrade: boolean;
  }): Promise<void> {
    console.log("CONFIG", {
      isStable,
      oracle,
      tradeOracle,
      pricing: pricingConfig,
      allowSwap,
      allowTrade,
      fees,
      borrowRate,
      ratios,
    });

    await this.program.methods
      .addCustody({
        isStable,
        pricing: pricingConfig,
        allowSwap,
        allowTrade,
        fees,
        borrowRate,
        ratios,
        oracle: this.toLimitedStringBuffer(oracle),
        tradeOracle: this.toLimitedStringBuffer(tradeOracle),
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custody: this.getCustodyKey(poolName, tokenMint),
        custodyTokenAccount: this.getCustodyTokenAccountKey(
          poolName,
          tokenMint
        ),
        custodyTokenMint: tokenMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        oracle: this.oracle.publicKey,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async removeCustody(
    poolName: string,
    tokenMint: PublicKey,
    ratios: TokenRatio[]
  ): Promise<void> {
    await this.program.methods
      .removeCustody({ ratios })
      .accountsStrict({
        admin: this.admin.publicKey,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custody: this.getCustodyKey(poolName, tokenMint),
        custodyTokenAccount: this.getCustodyTokenAccountKey(
          poolName,
          tokenMint
        ),
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([this.admin])
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async setPoolAumSoftCapUsd(poolName: string, aumSoftCapUsd: BN) {
    return this.program.methods
      .setPoolAumSoftCapUsd({
        aumSoftCapUsd,
      })
      .accountsStrict({
        admin: this.provider.wallet.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async setPoolLiquidityState(poolName: string, liquidityState: number) {
    return this.program.methods
      .setPoolLiquidityState({ liquidityState })
      .accountsStrict({
        admin: this.provider.wallet.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async setPoolAllowSwap(poolName: string, allowSwap: boolean) {
    return this.program.methods
      .setPoolAllowSwap({ allowSwap })
      .accountsStrict({
        admin: this.provider.wallet.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async setPoolAllowTrade(poolName: string, allowTrade: boolean) {
    return this.program.methods
      .setPoolAllowTrade({ allowTrade })
      .accountsStrict({
        admin: this.provider.wallet.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
      })
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getTokenAccountBalanceNullable(ata: PublicKey): Promise<BN | null> {
    try {
      return new BN(
        (
          await this.provider.connection.getTokenAccountBalance(ata)
        ).value.amount
      );
    } catch {
      return null;
    }
  }

  // Make sure the WSOL ATA is created and having enough tokens
  async createPrepareWSOLAccountInstructions({
    amount,
    owner,
    wsolATA,
  }: {
    amount: BN;
    owner: PublicKey;
    wsolATA: PublicKey;
  }) {
    const currentWSOLBalance =
      (await this.getTokenAccountBalanceNullable(wsolATA)) ?? new BN(0);

    const instructions: TransactionInstruction[] = [
      createAssociatedTokenAccountIdempotentInstruction(
        owner,
        wsolATA,
        owner,
        NATIVE_MINT
      ),
    ];

    // enough WSOL available
    if (amount.isZero() || currentWSOLBalance >= amount) {
      return instructions;
    }

    return [
      ...instructions,

      // Transfer missing tokens
      SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: wsolATA,
        lamports: amount.sub(currentWSOLBalance).toNumber(),
      }),

      // Sync
      createSyncNativeInstruction(wsolATA),
    ];
  }

  async addLiquidity({
    poolName,
    tokenMint,
    amountIn,
    minLpAmountOut,
    oraclePrices,
  }: {
    poolName: string;
    tokenMint: PublicKey;
    amountIn: BN;
    minLpAmountOut: BN;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<void> {
    const lpTokenMint = this.getPoolLpTokenKey(poolName);

    const lpStaking = this.findProgramAddress("staking", [
      lpTokenMint,
    ]).publicKey;

    const lpTokenAccount = await getAssociatedTokenAddress(
      lpTokenMint,
      this.provider.wallet.publicKey
    );

    const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
      units: 500_000,
    });

    const preInstructions: TransactionInstruction[] = [modifyComputeUnits];

    // init LP token ATA if it doesn't exist
    if (!(await this.provider.connection.getAccountInfo(lpTokenAccount))) {
      preInstructions.push(
        createAssociatedTokenAccountInstruction(
          this.provider.wallet.publicKey,
          lpTokenAccount,
          this.provider.wallet.publicKey,
          lpTokenMint
        )
      );
    }

    // Make sure there are enough WSOL available, if adding WSOL
    // Don't do it as preInstructions due to the transaction being too large
    // i.e Error: Transaction too large: 1258 > 1232
    if (tokenMint.equals(NATIVE_MINT)) {
      const wsolATA = await getAssociatedTokenAddress(
        NATIVE_MINT,
        this.provider.wallet.publicKey
      );

      const ix = await this.createPrepareWSOLAccountInstructions({
        amount: amountIn,
        owner: this.provider.wallet.publicKey,
        wsolATA,
      });

      const tx = new Transaction();

      tx.add(...ix);
      tx.recentBlockhash = (
        await this.provider.connection.getLatestBlockhash()
      ).blockhash;
      tx.feePayer = this.provider.wallet.publicKey;

      const signedTransaction = await this.provider.wallet.signTransaction(tx);

      const txId = await this.provider.connection.sendRawTransaction(
        signedTransaction.serialize()
      );

      const confirmationStatus =
        await this.provider.connection.confirmTransaction(txId, "confirmed");

      if (confirmationStatus.value.err) {
        console.error(`Transaction failed: ${confirmationStatus.value.err}`);
      } else {
        console.log(`Transaction succeeded: ${txId}`);
      }
    }

    await this.program.methods
      .addLiquidity({
        amountIn, minLpAmountOut,
        oraclePrices,
      })
      .accountsStrict({
        owner: this.provider.wallet.publicKey,
        fundingAccount: await getAssociatedTokenAddress(
          tokenMint,
          this.provider.wallet.publicKey
        ),
        lpTokenAccount,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custodyTokenAccount: this.getCustodyTokenAccountKey(
          poolName,
          tokenMint
        ),
        lpTokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        adrenaProgram: this.program.programId,
        oracle: this.oracle.publicKey,
        custody: this.getCustodyKey(poolName, tokenMint),
        lpStaking,
      })
      .remainingAccounts(await this.getCustodyMetas(poolName))
      .preInstructions(preInstructions)
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async updatePoolAum({
    poolName,
    oraclePrices,
  }: {
    poolName: string;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<string> {
    return this.program.methods
      .updatePoolAum({ oraclePrices }) // Provide the required argument
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        payer: this.provider.wallet.publicKey,
        oracle: this.oracle.publicKey,
      })
      .remainingAccounts(await this.getCustodyMetas(poolName))
      .rpc()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  findCustodyAddress(mint: PublicKey, pool: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from('custody'),
        pool.toBuffer(),
        mint.toBuffer(),
      ],
      this.program.programId,
    )[0];
  }

  findCustodyTokenAccountAddress(mint: PublicKey, pool: PublicKey) {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from('custody_token_account'),
        pool.toBuffer(),
        mint.toBuffer(),
      ],
      this.program.programId,
    )[0];
  }

  findATAAddressSync(
    wallet: PublicKey,
    mint: PublicKey,
  ): PublicKey {
    return PublicKey.findProgramAddressSync(
      [wallet.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID,
    )[0];
  }

  // i.e percentage = -2 (for -2%)
  // i.e percentage = 5 (for 5%)
  applySlippage(nb: BN, percentage: number): BN {
    const negative = percentage < 0 ? true : false;

    // Do x10_000 so percentage can be up to 4 decimals
    const percentageBN = new BN(
      (negative ? percentage * -1 : percentage) * 10_000,
    );

    const delta = nb.mul(percentageBN).divRound(new BN(10_000 * 100));

    return negative ? nb.sub(delta) : nb.add(delta);
  }

  findPositionAddress(
    owner: PublicKey,
    custody: PublicKey,
    side: 'long' | 'short',
    pool: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from('position'),
        owner.toBuffer(),
        pool.toBuffer(),
        custody.toBuffer(),
        Buffer.from([
          {
            long: 1,
            short: 2,
          }[side],
        ]),
      ],
      this.program.programId,
    )[0];
  }

  async closePositionLong({
    positionKey,
    price,
    oraclePrices,
  }: {
    positionKey: PublicKey;
    price: BN;
    oraclePrices: ChaosLabsBatchPrices;
  }) {
    const preInstructions: TransactionInstruction[] = [];
    const postInstructions: TransactionInstruction[] = [];

    const position = await this.program.account.position.fetch(positionKey);
    const custody = await this.program.account.custody.fetch(position.custody);
    const collateralCustody = await this.program.account.custody.fetch(position.collateralCustody);

    const custodyTokenAccount = this.findCustodyTokenAccountAddress(
      custody.mint,
      position.pool,
    );

    const receivingAccount = this.findATAAddressSync(position.owner, collateralCustody.mint);

    return this.program.methods
      .closePositionLong({
        price,
        oraclePrices,
      })
      .accountsStrict({
        owner: position.owner,
        receivingAccount,
        transferAuthority: this.authority.publicKey,
        pool: position.pool,
        position: positionKey,
        custody: position.custody,
        custodyTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        cortex: this.cortex.publicKey,
        adrenaProgram: this.program.programId,
        caller: position.owner,
        userProfile: null,
        referrerProfile: null,
        oracle: this.oracle.publicKey,
      })
      .preInstructions(preInstructions)
      .postInstructions(postInstructions)
      .rpc();
  }

  async openPositionWithSwapLong({
    poolName,
    mint,
    collateralMint,
    collateralAmount,
    price,
    leverage,
    oraclePrices,
  }: {
    poolName: string;
    mint: PublicKey;
    collateralMint: PublicKey;
    collateralAmount: BN;
    price: BN;
    leverage: number;
    oraclePrices: ChaosLabsBatchPrices;
  }) {
    const owner = this.provider.wallet.publicKey;
    const poolKey = this.getPoolKey(poolName);
    const receivingCustody = this.findCustodyAddress(collateralMint, poolKey);
    const receivingCustodyTokenAccount =
      this.findCustodyTokenAccountAddress(collateralMint, poolKey);

      console.log('>> PoolKey', poolKey.toBase58());

    const collateralAccount = this.findATAAddressSync(owner, mint);

    // Principal custody is the custody of the targeted token
    // i.e open a 1 ETH long position, principal custody is ETH
    const principalCustody = this.findCustodyAddress(mint, poolKey);
    const principalCustodyTokenAccount =
      this.findCustodyTokenAccountAddress(mint, poolKey);

    const fundingAccount = this.findATAAddressSync(owner, collateralMint);
    const position = this.findPositionAddress(owner, principalCustody, 'long', poolKey);

    // Think and use proper slippage, for now use 0.3%
    const priceWithSlippage = this.applySlippage(price, 0.3);

    const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
      units: 500_000,
    });
    const preInstructions: TransactionInstruction[] = [modifyComputeUnits];

    return this.program.methods
      .openOrIncreasePositionWithSwapLong({
        price: priceWithSlippage,
        collateral: collateralAmount,
        leverage,
        oraclePrices,
      })
      .accountsStrict({
        owner,
        payer: owner,
        fundingAccount,
        collateralAccount,
        receivingCustody,
        receivingCustodyTokenAccount,
        principalCustody,
        principalCustodyTokenAccount,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        position,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        adrenaProgram: this.program.programId,
        pool: poolKey,
        oracle: this.oracle.publicKey,
      })
      .preInstructions(preInstructions)
      .rpc();
  }

  async addVest(
    beneficiaryWalet: PublicKey,
    amount: BN,
    unlockStartTimestamp: BN,
    unlockEndTimestamp: BN,
    voteMultiplier: number
  ): Promise<string> {
    const cortexAccount = await this.program.account.cortex.fetch(
      this.cortex.publicKey
    );

    /*
    pub enum BucketName {
        CoreContributor = 0,
        Foundation = 1,
        #[default]
        Ecosystem = 2,
    }
    */

    return this.program.methods
      .addVest({
        amount,
        unlockStartTimestamp,
        unlockEndTimestamp,
        originBucket: 0, // CoreContributor
        voteMultiplier,
      })
      .accountsStrict({
        admin: this.admin.publicKey,
        owner: beneficiaryWalet,
        payer: this.provider.wallet.publicKey,
        transferAuthority: this.authority.publicKey,
        cortex: this.cortex.publicKey,
        vestRegistry: this.vestRegistry.publicKey,
        vest: this.getVestPda(beneficiaryWalet).publicKey,
        lmTokenMint: this.lmTokenMint.publicKey,
        governanceTokenMint: this.governanceTokenMint.publicKey,
        governanceRealm: cortexAccount.governanceRealm,
        governanceRealmConfig: await getRealmConfigAddress(
          governanceProgram,
          cortexAccount.governanceRealm
        ),
        governanceGoverningTokenHolding: await getGoverningTokenHoldingAddress(
          governanceProgram,
          cortexAccount.governanceRealm,
          this.governanceTokenMint.publicKey
        ),
        governanceGoverningTokenOwnerRecord: await getTokenOwnerRecordAddress(
          governanceProgram,
          cortexAccount.governanceRealm,
          this.governanceTokenMint.publicKey,
          beneficiaryWalet
        ),
        governanceProgram,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();
  }

  async getAddLiquidityAmountAndFee({
    poolName,
    tokenMint,
    amount,
    oraclePrices,
  }: {
    poolName: string,
    tokenMint: PublicKey,
    amount: BN,
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<AmountAndFee> {
    return this.program.methods
      .getAddLiquidityAmountAndFee({
        amountIn: amount,
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custody: this.getCustodyKey(poolName, tokenMint),
        lpTokenMint: this.getPoolLpTokenKey(poolName),
        oracle: this.oracle.publicKey,
      })
      .remainingAccounts(await this.getCustodyMetas(poolName))
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getRemoveLiquidityAmountAndFee({
    poolName,
    tokenMint,
    lpAmount,
    oraclePrices,
  }: {
    poolName: string;
    tokenMint: PublicKey;
    lpAmount: BN;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<AmountAndFee> {
    return this.program.methods
      .getRemoveLiquidityAmountAndFee({
        lpAmountIn: lpAmount,
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custody: this.getCustodyKey(poolName, tokenMint),
        lpTokenMint: this.getPoolLpTokenKey(poolName),
        oracle: this.oracle.publicKey,
      })
      .remainingAccounts(await this.getCustodyMetas(poolName))
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getEntryPriceAndFee({
    poolName,
    tokenMint,
    collateralMint,
    collateral,
    leverage,
    side,
    oraclePrices,
  }: {
    poolName: string;
    tokenMint: PublicKey;
    collateralMint: PublicKey;
    collateral: BN;
    leverage: number;
    side: PositionSide;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<NewPositionPricesAndFee> {
    return this.program.methods
      .getEntryPriceAndFee({
        collateral,
        leverage,
        side: side === "long" ? 1 : 2, // Long = 1, Short = 2
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        custody: this.getCustodyKey(poolName, tokenMint),
        collateralCustody: this.getCustodyKey(poolName, collateralMint),
        oracle: this.oracle.publicKey,
      })
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getLiquidationPrice({
    wallet,
    poolName,
    tokenMint,
    collateralMint,
    side,
    addCollateral,
    removeCollateral,
    oraclePrices,
  }: {
    wallet: PublicKey;
    poolName: string;
    tokenMint: PublicKey;
    collateralMint: PublicKey;
    side: PositionSide;
    addCollateral: BN;
    removeCollateral: BN;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<BN> {
    return this.program.methods
      .getLiquidationPrice({
        addCollateral,
        removeCollateral,
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        position: this.getPositionKey(wallet, poolName, tokenMint, side),
        custody: this.getCustodyKey(poolName, tokenMint),
        collateralCustody: this.getCustodyKey(poolName, collateralMint),
        oracle: this.oracle.publicKey,
      })
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getLiquidationState({
    wallet,
    poolName,
    tokenMint,
    collateralMint,
    side,
    oraclePrices,
  }: {
    wallet: PublicKey;
    poolName: string;
    tokenMint: PublicKey;
    collateralMint: PublicKey;
    side: PositionSide;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<number> {
    return this.program.methods
      .getLiquidationState({
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        position: this.getPositionKey(wallet, poolName, tokenMint, side),
        custody: this.getCustodyKey(poolName, tokenMint),
        collateralCustody: this.getCustodyKey(poolName, collateralMint),
        oracle: this.oracle.publicKey,
      })
      .view()
      .catch(() => {
        console.error(
          "Error getting liquidation state for position : " +
            this.getPositionKey(wallet, poolName, tokenMint, side).toBase58()
        );
      });
  }

  async getPnl({
    wallet,
    poolName,
    tokenMint,
    collateralMint,
    side,
    oraclePrices,
  }: {
    wallet: PublicKey;
    poolName: string;
    tokenMint: PublicKey;
    collateralMint: PublicKey;
    side: PositionSide;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<ProfitAndLoss> {
    return this.program.methods
      .getPnl({
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        position: this.getPositionKey(wallet, poolName, tokenMint, side),
        custody: this.getCustodyKey(poolName, tokenMint),
        collateralCustody: this.getCustodyKey(poolName, collateralMint),
        oracle: this.oracle.publicKey,
      })
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getSwapAmountAndFees({
    poolName,
    tokenMintIn,
    tokenMintOut,
    amountIn,
    oraclePrices,
  }: {
    poolName: string;
    tokenMintIn: PublicKey;
    tokenMintOut: PublicKey;
    amountIn: BN;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<SwapAmountAndFees> {
    return this.program.methods
      .getSwapAmountAndFees({
        amountIn,
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        receivingCustody: this.getCustodyKey(poolName, tokenMintIn),
        dispensingCustody: this.getCustodyKey(poolName, tokenMintOut),
        oracle: this.oracle.publicKey,
      })
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getAum({
    poolName,
    oraclePrices,
  }: {
    poolName: string;
    oraclePrices: ChaosLabsBatchPrices;
  }): Promise<BN> {
    return this.program.methods
      .getAssetsUnderManagement({
        oraclePrices,
      })
      .accountsStrict({
        cortex: this.cortex.publicKey,
        pool: this.getPoolKey(poolName),
        oracle: this.oracle.publicKey,
      })
      .remainingAccounts(await this.getCustodyMetas(poolName))
      .view()
      .catch((err) => {
        console.error(err);
        throw err;
      });
  }

  async getAssetPrices(): Promise<ChaosLabsBatchPrices> {
    const res = await fetch("https://datapi.adrena.xyz/last-trading-prices", {
      method: "GET",
      headers: {
        accept: "application/json",
      },
    });
  
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    }
  
    const data = (await res.json()) as {
      data: {
        prices: {
          symbol: string;
          feed_id: number;
          price: number;
          timestamp: number;
          exponent: number;
        }[];
        latest_date: string;
        latest_timestamp: number;
        signature: string;
        recovery_id: number;
      };
    };

    console.log(">>> ChaosLabsBatchPrices", {
      prices: data.data.prices.map((price) => ({
        feedId: price.feed_id,
        price: price.price,
        timestamp: price.timestamp,
      })),
      signature: data.data.signature,
      recoveryId: data.data.recovery_id,
    });

    return {
      prices: data.data.prices.map((price) => ({
        feedId: price.feed_id,
        price: new BN(price.price),
        timestamp: new BN(price.timestamp),
      })),
      signature: this.hexToBytes(data.data.signature),
      recoveryId: data.data.recovery_id,
    } as unknown as ChaosLabsBatchPrices;
  }

  hexToBytes(hex: string): Uint8Array {
    if (hex.length % 2 !== 0) throw new Error("Hex string must have even length");

    const bytes = new Uint8Array(hex.length / 2);

    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }

    return bytes;
  }
}
