import type { Program } from "@project-serum/anchor";
import type { AnchorTypes } from "@saberhq/anchor-contrib";

import type { QuarryOperatorIDL } from "../idls/quarry_operator";

export * from "../idls/quarry_operator";

export type QuarryOperatorTypes = AnchorTypes<
  QuarryOperatorIDL,
  {
    operator: OperatorData;
  }
>;

type Accounts = QuarryOperatorTypes["Accounts"];
export type OperatorData = Accounts["operator"];

export type QuarryOperatorError = QuarryOperatorTypes["Error"];
export type QuarryOperatorProgram = Program<QuarryOperatorIDL>;
