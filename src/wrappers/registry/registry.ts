import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { PublicKey } from "@solana/web3.js";
import { SystemProgram } from "@solana/web3.js";

import type { RegistryProgram } from "../../programs";
import type { QuarrySDK } from "../../sdk";
import { findQuarryAddress } from "../mine";
import { findRegistryAddress } from "./pda";

export class QuarryRegistry {
  readonly program: RegistryProgram;
  constructor(readonly sdk: QuarrySDK) {
    this.program = sdk.programs.Registry;
  }

  get provider(): Provider {
    return this.sdk.provider;
  }

  /**
   * Creates a new Registry.
   * @returns
   */
  async newRegistry({
    numQuarries,
    rewarderKey,
    payer = this.provider.wallet.publicKey,
  }: {
    numQuarries: number;
    rewarderKey: PublicKey;
    payer?: PublicKey;
  }): Promise<{ tx: TransactionEnvelope; registry: PublicKey }> {
    const [registry, bump] = await findRegistryAddress(
      rewarderKey,
      this.program.programId
    );
    const createRegistryTX = new TransactionEnvelope(this.provider, [
      this.program.instruction.newRegistry(numQuarries, bump, {
        accounts: {
          rewarder: rewarderKey,
          registry,
          payer,
          systemProgram: SystemProgram.programId,
        },
      }),
    ]);
    return {
      tx: createRegistryTX,
      registry,
    };
  }

  async syncQuarry({
    tokenMint,
    rewarderKey,
  }: {
    tokenMint: PublicKey;
    rewarderKey: PublicKey;
  }): Promise<TransactionEnvelope> {
    const [registry] = await findRegistryAddress(
      rewarderKey,
      this.program.programId
    );
    const [quarry] = await findQuarryAddress(
      rewarderKey,
      tokenMint,
      this.sdk.programs.Mine.programId
    );
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.syncQuarry({
        accounts: { quarry, registry },
      }),
    ]);
  }
}
