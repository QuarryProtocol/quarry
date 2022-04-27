import type { Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { QuarryRegistryIDL } from "../idls/quarry_registry";

export * from "../idls/quarry_registry";

export type RegistryTypes = AnchorTypes<
  QuarryRegistryIDL,
  {
    registry: RegistryData;
  }
>;

type Accounts = RegistryTypes["Accounts"];

export type RegistryData = Accounts["registry"];

export type RegistryError = RegistryTypes["Error"];

export type RegistryProgram = Program<QuarryRegistryIDL>;
