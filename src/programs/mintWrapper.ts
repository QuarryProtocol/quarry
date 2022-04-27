import type { Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { QuarryMintWrapperIDL } from "../idls/quarry_mint_wrapper";

export * from "../idls/quarry_mint_wrapper";

export type MintWrapperTypes = AnchorTypes<
  QuarryMintWrapperIDL,
  {
    mintWrapper: MintWrapperData;
    minter: MinterData;
  }
>;

type Accounts = MintWrapperTypes["Accounts"];

export type MintWrapperData = Accounts["mintWrapper"];
export type MinterData = Accounts["minter"];

export type MintWrapperError = MintWrapperTypes["Error"];

export type MintWrapperProgram = Program<QuarryMintWrapperIDL>;
