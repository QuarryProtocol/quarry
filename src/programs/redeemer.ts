import type { AnchorTypes } from "@saberhq/anchor-contrib";
import type { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { u64 } from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";

import type { QuarryRedeemerIDL } from "../idls/quarry_redeemer";

export * from "../idls/quarry_redeemer";

export type RedeemerTypes = AnchorTypes<
  QuarryRedeemerIDL,
  {
    redeemer: RedeemerData;
  }
>;

type Accounts = RedeemerTypes["Accounts"];
export type RedeemerData = Accounts["Redeemer"];

export type RedeemerError = RedeemerTypes["Error"];
export type RedeemerEvents = RedeemerTypes["Events"];
export type RedeemerProgram = RedeemerTypes["Program"];

export type PendingRedeemer = {
  bump: number;
  vaultTokenAccount: PublicKey;
  tx: TransactionEnvelope;
};

export type RedeemTokenArgs = {
  tokenAmount: u64;
  sourceAuthority: PublicKey;
  iouSource: PublicKey;
  redemptionDestination: PublicKey;
};
