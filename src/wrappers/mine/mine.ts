import type { Provider, TransactionEnvelope } from "@saberhq/solana-contrib";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { MineProgram } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { findRewarderAddress } from "./pda";
import { RewarderWrapper } from "./rewarder";

export class MineWrapper {
  constructor(public readonly sdk: QuarrySDK) {}

  get provider(): Provider {
    return this.sdk.provider;
  }

  get program(): MineProgram {
    return this.sdk.programs.Mine;
  }

  public async createRewarder({
    mintWrapper,
    baseKP = Keypair.generate(),
    authority = this.program.provider.wallet.publicKey,
  }: {
    mintWrapper: PublicKey;
    baseKP?: Keypair;
    authority?: PublicKey;
  }): Promise<{
    key: PublicKey;
    tx: TransactionEnvelope;
  }> {
    const [rewarderKey, bump] = await findRewarderAddress(
      baseKP.publicKey,
      this.program.programId
    );
    const mintWrapperData =
      await this.sdk.programs.MintWrapper.account.mintWrapper.fetch(
        mintWrapper
      );
    return {
      key: rewarderKey,
      tx: this.sdk.newTx(
        [
          this.program.instruction.newRewarder(bump, {
            accounts: {
              base: baseKP.publicKey,
              authority,
              rewarder: rewarderKey,
              payer: this.program.provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
              clock: SYSVAR_CLOCK_PUBKEY,
              mintWrapper,
              mintWrapperProgram: this.sdk.programs.MintWrapper.programId,
              rewardsTokenMint: mintWrapperData.tokenMint,
            },
          }),
        ],
        [baseKP]
      ),
    };
  }

  /**
   * Loads the rewarder wrapper.
   * @param rewarder
   * @returns
   */
  public async loadRewarderWrapper(
    rewarder: PublicKey
  ): Promise<RewarderWrapper> {
    const rewarderData = await this.program.account.rewarder.fetch(rewarder);
    return new RewarderWrapper(this, rewarder, rewarderData);
  }
}
