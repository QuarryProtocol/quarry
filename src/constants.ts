import { buildCoderMap } from "@saberhq/anchor-contrib";
import { PublicKey } from "@solana/web3.js";

import { QuarryMineJSON } from "./idls/quarry_mine";
import { QuarryMintWrapperJSON } from "./idls/quarry_mint_wrapper";
import { QuarryRedeemerJSON } from "./idls/quarry_redeemer";
import type {
  MineProgram,
  MineTypes,
  MintWrapperProgram,
  MintWrapperTypes,
  QuarryMergeMineProgram,
  QuarryMergeMineTypes,
  QuarryOperatorProgram,
  QuarryOperatorTypes,
} from "./programs";
import { QuarryMergeMineJSON, QuarryOperatorJSON } from "./programs";
import type { RedeemerProgram, RedeemerTypes } from "./programs/redeemer";
import type { RegistryProgram, RegistryTypes } from "./programs/registry";
import { QuarryRegistryJSON } from "./programs/registry";

/**
 * Types of all programs.
 */
export interface Programs {
  MergeMine: QuarryMergeMineProgram;
  Mine: MineProgram;
  MintWrapper: MintWrapperProgram;
  Operator: QuarryOperatorProgram;
  Redeemer: RedeemerProgram;
  Registry: RegistryProgram;
}

/**
 * Quarry program addresses.
 */
export const QUARRY_ADDRESSES = {
  MergeMine: new PublicKey("QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto"),
  Mine: new PublicKey("QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB"),
  MintWrapper: new PublicKey("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV"),
  Operator: new PublicKey("QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz"),
  Redeemer: new PublicKey("QRDxhMw1P2NEfiw5mYXG79bwfgHTdasY2xNP76XSea9"),
  Registry: new PublicKey("QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc"),
};

/**
 * Quarry program IDLs.
 */
export const QUARRY_IDLS = {
  MergeMine: QuarryMergeMineJSON,
  Mine: QuarryMineJSON,
  MintWrapper: QuarryMintWrapperJSON,
  Operator: QuarryOperatorJSON,
  Redeemer: QuarryRedeemerJSON,
  Registry: QuarryRegistryJSON,
};

/**
 * Quarry program IDLs.
 */
export const QUARRY_CODERS = buildCoderMap<{
  MergeMine: QuarryMergeMineTypes;
  Mine: MineTypes;
  MintWrapper: MintWrapperTypes;
  Operator: QuarryOperatorTypes;
  Redeemer: RedeemerTypes;
  Registry: RegistryTypes;
}>(QUARRY_IDLS, QUARRY_ADDRESSES);

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
