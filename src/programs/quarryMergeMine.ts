import type { IdlAccounts, Program } from "@project-serum/anchor";
import type { AllInstructionsMap } from "@project-serum/anchor/dist/esm/program/namespace/types";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { QuarryMergeMineIDL } from "../idls/quarry_merge_mine";
import type { AccountMaps } from "../wrappers/mine/miner";

export * from "../idls/quarry_merge_mine";

export type QuarryMergeMineTypes = AnchorTypes<
  QuarryMergeMineIDL,
  {
    mergePool: MergePoolData;
    mergeMiner: MergeMinerData;
  }
>;

type Accounts = IdlAccounts<QuarryMergeMineIDL>;
export type MergePoolData = Accounts["mergePool"];
export type MergeMinerData = Accounts["mergeMiner"];

export type QuarryMergeMineError = QuarryMergeMineTypes["Error"];
export type QuarryMergeMineProgram = Program<QuarryMergeMineIDL>;

export type QuarryStakeAccounts = AccountMaps<
  AllInstructionsMap<QuarryMergeMineIDL>["stakePrimaryMiner"]["accounts"][2]
>;
