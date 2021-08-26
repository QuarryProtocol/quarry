import { PublicKey } from "@solana/web3.js";

import { QuarryMineJSON } from "./idls/quarry_mine";
import { QuarryMintWrapperJSON } from "./idls/quarry_mint_wrapper";
import type { MineProgram, MintWrapperProgram } from "./programs";

export interface Programs {
  MintWrapper: MintWrapperProgram;
  Mine: MineProgram;
}

// See `Anchor.toml` for all addresses.
export const QUARRY_ADDRESSES = {
  MintWrapper: new PublicKey("QMWVettd5nC2Y9nSkXa4pznj2dMfBg5oqvwc4kV8Sa6"),
  Mine: new PublicKey("QMNFUvncKBh11ZgEwYtoup3aXvuVxt6fzrcsjk2cjpM"),
};

export const QUARRY_IDLS = {
  MintWrapper: QuarryMintWrapperJSON,
  Mine: QuarryMineJSON,
};
