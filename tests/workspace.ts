import "chai-bn";

import type { Idl } from "@project-serum/anchor";
import * as anchor from "@project-serum/anchor";
import { chaiSolana } from "@saberhq/chai-solana";
import { SolanaProvider } from "@saberhq/solana-contrib";
import chai, { assert } from "chai";

import type { Programs } from "../src";
import { QuarrySDK } from "../src";

chai.use(chaiSolana);

export type Workspace = Programs;

export const makeSDK = (): QuarrySDK => {
  const anchorProvider = anchor.Provider.env();
  anchor.setProvider(anchorProvider);

  const provider = SolanaProvider.load({
    connection: anchorProvider.connection,
    sendConnection: anchorProvider.connection,
    wallet: anchorProvider.wallet,
    opts: anchorProvider.opts,
  });
  return QuarrySDK.load({
    provider,
  });
};

type IDLError = NonNullable<Idl["errors"]>[number];

export const assertError = (error: IDLError, other: IDLError): void => {
  assert.strictEqual(error.code, other.code);
  assert.strictEqual(error.msg, other.msg);
};
