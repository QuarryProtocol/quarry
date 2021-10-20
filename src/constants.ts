import { PublicKey } from "@solana/web3.js";

import { QuarryMineJSON } from "./idls/quarry_mine";
import { QuarryMintWrapperJSON } from "./idls/quarry_mint_wrapper";
import { QuarryRedeemerJSON } from "./idls/quarry_redeemer";
import type {
  MineProgram,
  MintWrapperProgram,
  QuarryMergeMineProgram,
  QuarryOperatorProgram,
} from "./programs";
import { QuarryMergeMineJSON, QuarryOperatorJSON } from "./programs";
import type { RedeemerProgram } from "./programs/redeemer";
import type { RegistryProgram } from "./programs/registry";
import { QuarryRegistryJSON } from "./programs/registry";

export interface Programs {
  MergeMine: QuarryMergeMineProgram;
  Mine: MineProgram;
  MintWrapper: MintWrapperProgram;
  Operator: QuarryOperatorProgram;
  Redeemer: RedeemerProgram;
  Registry: RegistryProgram;
}

// See `Anchor.toml` for all addresses.
export const QUARRY_ADDRESSES = {
  MergeMine: new PublicKey("QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto"),
  Mine: new PublicKey("QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB"),
  MintWrapper: new PublicKey("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV"),
  Operator: new PublicKey("QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz"),
  Redeemer: new PublicKey("QRDxhMw1P2NEfiw5mYXG79bwfgHTdasY2xNP76XSea9"),
  Registry: new PublicKey("QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc"),
};

export const QUARRY_IDLS = {
  MergeMine: QuarryMergeMineJSON,
  Mine: QuarryMineJSON,
  MintWrapper: QuarryMintWrapperJSON,
  Operator: QuarryOperatorJSON,
  Redeemer: QuarryRedeemerJSON,
  Registry: QuarryRegistryJSON,
};

/**
 * Recipient of protocol fees.
 */
export const QUARRY_FEE_TO = new PublicKey(
  "4MMZH3ih1aSty2nx4MC3kSR94Zb55XsXnqb5jfEcyHWQ"
);

/**
 * Sets the protocol fees.
 */
export const QUARRY_FEE_SETTER = new PublicKey(
  "4MMZH3ih1aSty2nx4MC3kSR94Zb55XsXnqb5jfEcyHWQ"
);
