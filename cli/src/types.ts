import { MethodsNamespace, IdlTypes, IdlAccounts } from "@coral-xyz/anchor";
import { Adrena } from "../../target/types/adrena";
import { PublicKey } from "@solana/web3.js";

export type PositionSide = "long" | "short";

export type Methods = MethodsNamespace<Adrena>;
export type Accounts = IdlAccounts<Adrena>;
export type Types = IdlTypes<Adrena>;

export type InitOneParams = Types["InitOneParams"];
export type OraclePricesSetup = Types["OraclePricesSetup"];

export type InitStakingOneParams = Types["InitStakingOneParams"];

export type AddPoolPartOneParams = Types["AddPoolPartOneParams"];

export type PricingParams = Types["PricingParams"];
export type Fees = Types["Fees"];
export type BorrowRateParams = Types["BorrowRateParams"];
export type TokenRatio = Types["TokenRatios"];
export type AmountAndFee = Types["AmountAndFee"];
export type NewPositionPricesAndFee = Types["NewPositionPricesAndFee"];
export type ExitPriceAndFee = Types["ExitPriceAndFee"];
export type ProfitAndLoss = Types["ProfitAndLoss"];
export type SwapAmountAndFees = Types["SwapAmountAndFees"];
export type ChaosLabsBatchPrices = Types["ChaosLabsBatchPrices"];

export type Custody = Accounts["custody"];
export type Pool = Accounts["pool"];
export type Position = Accounts["position"];
export type Cortex = Accounts["cortex"];

export type LimitedString = Types['LimitedString'];

export type CustodyExtended = Custody & {
  pubkey: PublicKey;
};

export type PositionExtended = Position & {
  pubkey: PublicKey;
};
