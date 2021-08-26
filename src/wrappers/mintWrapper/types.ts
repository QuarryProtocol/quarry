import type { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { PublicKey } from "@solana/web3.js";

export interface PendingMintWrapper {
  mintWrapper: PublicKey;
  tx: TransactionEnvelope;
}

export interface PendingMintAndWrapper {
  mint: PublicKey;
  mintWrapper: PublicKey;
  tx: TransactionEnvelope;
}
