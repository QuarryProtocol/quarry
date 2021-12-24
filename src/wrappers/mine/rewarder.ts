import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { Token, u64 } from "@saberhq/token-utils";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { MineProgram, RewarderData } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import type { MineWrapper } from ".";
import { findQuarryAddress } from "./pda";
import { QuarryWrapper } from "./quarry";
import type { PendingQuarry } from "./types";

export class RewarderWrapper {
  readonly sdk: QuarrySDK;
  readonly program: MineProgram;

  constructor(
    readonly mineWrapper: MineWrapper,
    readonly rewarderKey: PublicKey,
    readonly rewarderData: RewarderData
  ) {
    this.sdk = mineWrapper.sdk;
    this.program = mineWrapper.program;
  }

  /**
   * Gets the quarry associated with the given token.
   * @param token
   * @returns
   */
  async getQuarry(token: Token): Promise<QuarryWrapper> {
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
  async createQuarry({
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
        unusedClock: SYSVAR_CLOCK_PUBKEY,
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
   * Updates annual rewards rate on the Rewarder.
   * One must sync after this.
   * @param param0
   */
  setAnnualRewards({
    newAnnualRate,
    authority = this.program.provider.wallet.publicKey,
  }: {
    newAnnualRate: u64;
    authority?: PublicKey;
  }): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setAnnualRewards(newAnnualRate, {
        accounts: {
          auth: {
            rewarder: this.rewarderKey,
            authority,
          },
        },
      }),
    ]);
  }

  /**
   * Updates to annual rewards rate on the quarry, and update rewards on quarries assocated with each mint provided.
   * @param param0
   */
  async setAndSyncAnnualRewards(
    newAnnualRate: u64,
    mints: PublicKey[]
  ): Promise<TransactionEnvelope> {
    const tx = await this.syncQuarryRewards(mints);
    return this.setAnnualRewards({ newAnnualRate }).combine(tx);
  }

  /**
   * Synchronizes quarry rewards.
   * @param mints
   * @returns
   */
  async syncQuarryRewards(mints: PublicKey[]): Promise<TransactionEnvelope> {
    const instructions: TransactionInstruction[] = [];
    await Promise.all(
      mints.map(async (m) => {
        const quarry = await this.getQuarryKeyForMint(m);
        instructions.push(
          this.program.instruction.updateQuarryRewards({
            accounts: {
              rewarder: this.rewarderKey,
              quarry,
            },
          })
        );
      })
    );
    return this.sdk.newTx(instructions);
  }

  /**
   * Transfers the authority to a different account.
   * @param param0
   */
  transferAuthority({
    authority = this.sdk.provider.wallet.publicKey,
    nextAuthority,
  }: {
    authority?: PublicKey;
    nextAuthority: PublicKey;
  }): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.transferAuthority(nextAuthority, {
        accounts: {
          authority,
          rewarder: this.rewarderKey,
        },
      }),
    ]);
  }

  /**
   * Sets timestamp on when rewards will cease
   */
  setFamine({
    newFamineTs,
    quarry,
    authority = this.sdk.provider.wallet.publicKey,
  }: {
    newFamineTs: u64;
    quarry: PublicKey;
    authority?: PublicKey;
  }): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setFamine(newFamineTs, {
        accounts: {
          auth: {
            authority,
            rewarder: this.rewarderKey,
          },
          quarry,
        },
      }),
    ]);
  }
}
