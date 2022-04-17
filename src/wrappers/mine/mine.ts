import type {
  AugmentedProvider,
  TransactionEnvelope,
} from "@saberhq/solana-contrib";
import { getOrCreateATA, TOKEN_PROGRAM_ID } from "@saberhq/token-utils";
import type {
  PublicKey,
  Signer,
  TransactionInstruction,
} from "@solana/web3.js";
import { Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import { QUARRY_CODERS } from "../../constants";
import type { MineProgram } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { findRewarderAddress } from "./pda";
import { RewarderWrapper } from "./rewarder";

export class MineWrapper {
  constructor(readonly sdk: QuarrySDK) {}

  get provider(): AugmentedProvider {
    return this.sdk.provider;
  }

  get program(): MineProgram {
    return this.sdk.programs.Mine;
  }

  /**
   *
   * @deprecated Use {@link createRewarder}.
   * @param param0
   * @returns
   */
  async createRewarderV1({
    mintWrapper,
    baseKP = Keypair.generate(),
    authority = this.provider.wallet.publicKey,
  }: {
    mintWrapper: PublicKey;
    baseKP?: Signer;
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
      QUARRY_CODERS.MintWrapper.accounts.mintWrapper.parse(
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
      tx: this.provider.newTX(
        [
          createATAInstruction,
          this.program.instruction.newRewarder(bump, {
            accounts: {
              base: baseKP.publicKey,
              initialAuthority: authority,
              rewarder: rewarderKey,
              payer: this.provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
              unusedAccount: SYSVAR_CLOCK_PUBKEY,
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
   * Creates a new Rewarder.
   * @param param0
   * @returns
   */
  async createRewarder({
    mintWrapper,
    baseKP = Keypair.generate(),
    authority = this.provider.wallet.publicKey,
  }: {
    mintWrapper: PublicKey;
    baseKP?: Signer;
    authority?: PublicKey;
  }): Promise<{
    key: PublicKey;
    tx: TransactionEnvelope;
  }> {
    const [rewarderKey] = await findRewarderAddress(
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
      QUARRY_CODERS.MintWrapper.accounts.mintWrapper.parse(
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
      tx: this.provider.newTX(
        [
          createATAInstruction,
          this.program.instruction.newRewarderV2({
            accounts: {
              base: baseKP.publicKey,
              initialAuthority: authority,
              rewarder: rewarderKey,
              payer: this.provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
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

  /**
   * Rescue stuck tokens in a miner.
   * @returns
   */
  async rescueTokens({
    mint,
    miner,
    minerTokenAccount,
    owner = this.provider.wallet.publicKey,
  }: {
    mint: PublicKey;
    miner: PublicKey;
    minerTokenAccount: PublicKey;
    owner?: PublicKey;
  }): Promise<TransactionEnvelope> {
    const instructions: TransactionInstruction[] = [];
    const { address: destinationTokenAccount, instruction: ataInstruction } =
      await getOrCreateATA({
        provider: this.provider,
        mint,
        owner,
      });
    if (ataInstruction) {
      instructions.push(ataInstruction);
    }

    instructions.push(
      this.program.instruction.rescueTokens({
        accounts: {
          authority: owner,
          miner,
          minerTokenAccount,
          destinationTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      })
    );

    return this.sdk.newTx(instructions);
  }
}
