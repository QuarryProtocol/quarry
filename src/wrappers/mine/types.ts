import type { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { PublicKey } from "@solana/web3.js";

import type { MinerWrapper } from "./miner";

export interface PendingRewarder {
  rewarder: PublicKey;
  base: PublicKey;
  tx: TransactionEnvelope;
}

export interface PendingQuarry {
  rewarder: PublicKey;
  quarry: PublicKey;
  tx: TransactionEnvelope;
}

export interface PendingMiner {
  miner: PublicKey;
  wrapper: MinerWrapper;
  tx: TransactionEnvelope;
}
