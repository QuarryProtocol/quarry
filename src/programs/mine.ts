import type { IdlAccounts, Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { QuarryMineIDL } from "../idls/quarry_mine";

export * from "../idls/quarry_mine";

export type MineTypes = AnchorTypes<
  QuarryMineIDL,
  {
    rewarder: RewarderData;
    quarry: QuarryData;
    miner: MinerData;
  }
>;

type MineAccounts = IdlAccounts<QuarryMineIDL>;

export type RewarderData = MineAccounts["rewarder"];
export type QuarryData = MineAccounts["quarry"];
export type MinerData = MineAccounts["miner"];

export type MineError = MineTypes["Error"];
export type MineEvents = MineTypes["Events"];
export type MineProgram = Program<QuarryMineIDL>;

export type ClaimEvent = MineEvents["ClaimEvent"];
export type StakeEvent = MineEvents["StakeEvent"];
export type WithdrawEvent = MineEvents["WithdrawEvent"];
export type QuarryRewardsUpdateEvent = MineEvents["QuarryRewardsUpdateEvent"];
