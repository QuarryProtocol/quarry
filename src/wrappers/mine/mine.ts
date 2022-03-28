import type { Provider, TransactionEnvelope } from "@saberhq/solana-contrib";
import { getOrCreateATA } from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { MintWrapperData } from "../../programs";
import type { MineProgram } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { findRewarderAddress } from "./pda";
import { RewarderWrapper } from "./rewarder";

export class MineWrapper {
  constructor(readonly sdk: QuarrySDK) {}

  get provider(): Provider {
    return this.sdk.provider;
  }

  get program(): MineProgram {
    return this.sdk.programs.Mine;
  }

  async createRewarder({
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

    const mintWrapperDataRaw = await this.provider.getAccountInfo(mintWrapper);
    if (!mintWrapperDataRaw) {
      throw new Error(
        `mint wrapper does not exist at ${mintWrapper.toString()}`
      );
    }

    const mintWrapperData =
      this.sdk.programs.MintWrapper.coder.accounts.decode<MintWrapperData>(
        "MintWrapper",
        mintWrapperDataRaw.accountInfo.data
      );

    const { address: claimFeeTokenAccount, instruction: createATAInstruction } =
      await getOrCreateATA({
        provider: this.provider,
        mint: mintWrapperData.tokenMint,
        owner: rewarderKey,
      });

    return {
      key: rewarderKey,
      tx: this.sdk.newTx(
        [
          ...(createATAInstruction ? [createATAInstruction] : []),
          this.program.instruction.newRewarder(bump, {
            accounts: {
              base: baseKP.publicKey,
              authority,
              rewarder: rewarderKey,
              payer: this.program.provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
              unusedClock: SYSVAR_CLOCK_PUBKEY,
              mintWrapper,
              rewardsTokenMint: mintWrapperData.tokenMint,
              claimFeeTokenAccount,
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
  async loadRewarderWrapper(rewarder: PublicKey): Promise<RewarderWrapper> {
    const rewarderData = await this.program.account.rewarder.fetch(rewarder);
    return new RewarderWrapper(this, rewarder, rewarderData);
  }
}
