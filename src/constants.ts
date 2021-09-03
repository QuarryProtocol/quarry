import { PublicKey } from "@solana/web3.js";

import { QuarryMineJSON } from "./idls/quarry_mine";
import { QuarryMintWrapperJSON } from "./idls/quarry_mint_wrapper";
import type { MineProgram, MintWrapperProgram } from "./programs";
import type { RegistryProgram } from "./programs/registry";
import { QuarryRegistryJSON } from "./programs/registry";

export interface Programs {
  MintWrapper: MintWrapperProgram;
  Mine: MineProgram;
  Registry: RegistryProgram;
}

// See `Anchor.toml` for all addresses.
export const QUARRY_ADDRESSES = {
  MintWrapper: new PublicKey("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV"),
  Mine: new PublicKey("QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB"),
  Registry: new PublicKey("QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc"),
};

export const QUARRY_IDLS = {
  MintWrapper: QuarryMintWrapperJSON,
  Mine: QuarryMineJSON,
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
