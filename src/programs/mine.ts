import type { Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { AnchorQuarryMine } from "../idls/quarry_mine";

export * from "../idls/quarry_mine";

export type MineTypes = AnchorTypes<
  AnchorQuarryMine,
  {
    rewarder: RewarderData;
    quarry: QuarryData;
    miner: MinerData;
  }
>;

type Accounts = MineTypes["Accounts"];
export type RewarderData = Accounts["rewarder"];
export type QuarryData = Accounts["quarry"];
export type MinerData = Accounts["miner"];

export type MineError = MineTypes["Error"];
export type MineEvents = MineTypes["Events"];
export type MineProgram = Omit<Program<AnchorQuarryMine>, "account"> &
  MineTypes["Program"];

export type ClaimEvent = MineEvents["ClaimEvent"];
export type StakeEvent = MineEvents["StakeEvent"];
export type WithdrawEvent = MineEvents["WithdrawEvent"];
export type QuarryRewardsUpdateEvent = MineEvents["QuarryRewardsUpdateEvent"];
