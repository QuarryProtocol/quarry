import type { Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";
import type { PublicKey } from "@solana/web3.js";

import type { AnchorQuarryMergeMine } from "../idls/quarry_merge_mine";

export * from "../idls/quarry_merge_mine";

export type QuarryMergeMineTypes = AnchorTypes<
  AnchorQuarryMergeMine,
  {
    mergePool: MergePoolData;
    mergeMiner: MergeMinerData;
  }
>;

type Accounts = QuarryMergeMineTypes["Accounts"];
export type MergePoolData = Accounts["mergePool"];
export type MergeMinerData = Accounts["mergeMiner"];

export type QuarryMergeMineError = QuarryMergeMineTypes["Error"];
export type QuarryMergeMineProgram = Omit<
  Program<AnchorQuarryMergeMine>,
  "account"
> &
  QuarryMergeMineTypes["Program"];

export type QuarryStakeAccounts = {
  [A in keyof Parameters<
    QuarryMergeMineProgram["instruction"]["stakePrimaryMiner"]["accounts"]
  >[0]["stake"]]: PublicKey;
};
