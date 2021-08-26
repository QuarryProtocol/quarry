import type { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { Token } from "@saberhq/token-utils";
import type { u64 } from "@solana/spl-token";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { MineProgram, RewarderData } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import type { MineWrapper } from ".";
import { findQuarryAddress } from "./pda";
import { QuarryWrapper } from "./quarry";
import type { PendingQuarry } from "./types";

export class RewarderWrapper {
  public readonly sdk: QuarrySDK;
  public readonly program: MineProgram;

  constructor(
    public readonly mineWrapper: MineWrapper,
    public readonly rewarderKey: PublicKey,
    public readonly rewarderData: RewarderData
  ) {
    this.sdk = mineWrapper.sdk;
    this.program = mineWrapper.program;
  }

  /**
   * Gets the quarry associated with the given token.
   * @param token
   * @returns
   */
  public async getQuarry(token: Token): Promise<QuarryWrapper> {
    const quarryKey = await this.getQuarryKey(token);
    return await QuarryWrapper.load({
      sdk: this.sdk,
      token,
      key: quarryKey,
    });
  }

  /**
   * Gets the public key of a quarry for a token.
   * @param token
   * @returns
   */
  async getQuarryKey(token: Token): Promise<PublicKey> {
    return await this.getQuarryKeyForMint(token.mintAccount);
  }

  /**
   * Gets the public key of a quarry for a token mint.
   * @param token
   * @returns
   */
  async getQuarryKeyForMint(mint: PublicKey): Promise<PublicKey> {
    const [quarryKey] = await findQuarryAddress(
      this.rewarderKey,
      mint,
      this.program.programId
    );
    return quarryKey;
  }

  /**
   * Creates a new quarry. Only the rewarder can call this.
   * @param param0
   * @returns
   */
  public async createQuarry({
    token,
    authority = this.program.provider.wallet.publicKey,
  }: {
    token: Token;
    authority?: PublicKey;
  }): Promise<PendingQuarry> {
    const [quarryKey, bump] = await findQuarryAddress(
      this.rewarderKey,
      token.mintAccount,
      this.program.programId
    );
    const ix = this.program.instruction.createQuarry(bump, {
      accounts: {
        quarry: quarryKey,
        auth: {
          authority,
          rewarder: this.rewarderKey,
        },
        tokenMint: token.mintAccount,
        payer: this.program.provider.wallet.publicKey,
        clock: SYSVAR_CLOCK_PUBKEY,
        systemProgram: SystemProgram.programId,
      },
    });

    return {
      rewarder: this.rewarderKey,
      quarry: quarryKey,
      tx: this.sdk.newTx([ix]),
    };
  }

  /**
   * Updates to daily rewards rate on the quarry, and update rewards on quarries assocated with each mint provided.
   * @param param0
   */
  public async setDailyRewards(
    newDailyRate: u64,
    mints: PublicKey[]
  ): Promise<TransactionEnvelope> {
    const authority = this.program.provider.wallet.publicKey;
    const tx = await this.syncQuarryRewards(mints);
    tx.instructions.unshift(
      this.program.instruction.setDailyRewards(newDailyRate, {
        accounts: {
          auth: {
            rewarder: this.rewarderKey,
            authority,
          },
          clock: SYSVAR_CLOCK_PUBKEY,
        },
      })
    );
    return tx;
  }

  /**
   * Synchronizes quarry rewards.
   * @param mints
   * @returns
   */
  public async syncQuarryRewards(
    mints: PublicKey[]
  ): Promise<TransactionEnvelope> {
    const instructions: TransactionInstruction[] = [];
    await Promise.all(
      mints.map(async (m) => {
        const quarry = await this.getQuarryKeyForMint(m);
        instructions.push(
          this.program.instruction.updateQuarryRewards({
            accounts: {
              rewarder: this.rewarderKey,
              quarry,
              clock: SYSVAR_CLOCK_PUBKEY,
            },
          })
        );
      })
    );
    return this.sdk.newTx(instructions);
  }
}
