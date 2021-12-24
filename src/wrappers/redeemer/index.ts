import { TransactionEnvelope } from "@saberhq/solana-contrib";
import {
  getATAAddress,
  getOrCreateATA,
  TOKEN_PROGRAM_ID,
} from "@saberhq/token-utils";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SystemProgram } from "@solana/web3.js";

import type { QuarrySDK } from "../..";
import type {
  PendingRedeemer,
  RedeemerData,
  RedeemerProgram,
  RedeemTokenArgs,
} from "../../programs/redeemer";
import { findRedeemerKey } from "./pda";

export * from "./pda";

export class RedeemerWrapper {
  constructor(
    readonly sdk: QuarrySDK,
    readonly iouMint: PublicKey,
    readonly redemptionMint: PublicKey,
    readonly key: PublicKey,
    readonly data: RedeemerData
  ) {}

  get program(): RedeemerProgram {
    return this.sdk.programs.Redeemer;
  }

  static async load({
    sdk,
    iouMint,
    redemptionMint,
  }: {
    sdk: QuarrySDK;
    iouMint: PublicKey;
    redemptionMint: PublicKey;
  }): Promise<RedeemerWrapper> {
    const [redeemer] = await findRedeemerKey({ iouMint, redemptionMint });
    const program = sdk.programs.Redeemer;
    const data = await program.account.redeemer.fetch(redeemer);
    return new RedeemerWrapper(sdk, iouMint, redemptionMint, redeemer, data);
  }

  static async createRedeemer({
    sdk,
    iouMint,
    redemptionMint,
  }: {
    sdk: QuarrySDK;
    iouMint: PublicKey;
    redemptionMint: PublicKey;
  }): Promise<PendingRedeemer> {
    const { provider } = sdk;
    const [redeemer, bump] = await findRedeemerKey({ iouMint, redemptionMint });
    const ata = await getOrCreateATA({
      provider,
      mint: redemptionMint,
      owner: redeemer,
    });
    return {
      bump,
      vaultTokenAccount: ata.address,
      tx: new TransactionEnvelope(sdk.provider, [
        ...(ata.instruction ? [ata.instruction] : []),
        sdk.programs.Redeemer.instruction.createRedeemer(bump, {
          accounts: {
            redeemer,
            iouMint,
            redemptionMint,
            payer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          },
        }),
      ]),
    };
  }

  /**
   * redeemTokensIx
   */
  async redeemTokensIx(args: RedeemTokenArgs): Promise<TransactionInstruction> {
    return this.program.instruction.redeemTokens(args.tokenAmount, {
      accounts: await this.getRedeemTokenAccounts(args),
    });
  }

  async redeemTokens(args: RedeemTokenArgs): Promise<TransactionEnvelope> {
    return new TransactionEnvelope(this.sdk.provider, [
      await this.redeemTokensIx(args),
    ]);
  }

  async getVaultAddress(): Promise<PublicKey> {
    return await getATAAddress({
      mint: this.redemptionMint,
      owner: this.key,
    });
  }

  async getRedeemTokenAccounts(
    args: Omit<RedeemTokenArgs, "tokenAmount">
  ): Promise<{
    redeemer: PublicKey;
    iouMint: PublicKey;
    redemptionMint: PublicKey;
    redemptionVault: PublicKey;
    tokenProgram: PublicKey;
    sourceAuthority: PublicKey;
    iouSource: PublicKey;
    redemptionDestination: PublicKey;
  }> {
    const { iouSource, redemptionDestination, sourceAuthority } = args;
    return {
      redeemer: this.key,
      iouMint: this.data.iouMint,
      redemptionMint: this.data.redemptionMint,
      redemptionVault: await getATAAddress({
        mint: this.data.redemptionMint,
        owner: this.key,
      }),
      tokenProgram: TOKEN_PROGRAM_ID,
      sourceAuthority,
      iouSource,
      redemptionDestination,
    };
  }
}
