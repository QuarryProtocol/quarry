import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { TokenAmount } from "@saberhq/token-utils";
import { getOrCreateATA } from "@saberhq/token-utils";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { MineProgram, MinerData } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { findMinterAddress } from "../mintWrapper/pda";
import type { QuarryWrapper } from "./quarry";
import type { PendingMiner } from "./types";

type MineUserStakeAccounts = Parameters<
  MineProgram["instruction"]["unusedStubClaimRewards"]["accounts"]
>[0]["stake"];

export class MinerWrapper {
  /**
   *
   * @param quarry
   * @param authority
   * @param minerKey
   * @param tokenVaultKey (associated w/ minerKey)
   * @param stakedTokenATA Staked token ATA (associated w/ authority)
   */
  constructor(
    public readonly quarry: QuarryWrapper,
    public readonly authority: PublicKey,
    public readonly minerKey: PublicKey,
    public readonly tokenVaultKey: PublicKey,
    public readonly stakedTokenATA: PublicKey
  ) {}

  /**
   * The program.
   */
  get program(): MineProgram {
    return this.quarry.program;
  }

  /**
   * The provider.
   */
  get provider(): Provider {
    return this.quarry.provider;
  }

  /**
   * The mining SDK.
   */
  get sdk(): QuarrySDK {
    return this.quarry.sdk;
  }

  /**
   * Creates the miner of the provided wallet.
   */
  public initialize(bump: number): PendingMiner {
    const instruction = this.program.instruction.createMiner(bump, {
      accounts: {
        authority: this.authority,
        miner: this.minerKey,
        quarry: this.quarry.key,
        systemProgram: SystemProgram.programId,
        payer: this.program.provider.wallet.publicKey,
        minerVault: this.tokenVaultKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMint: this.quarry.token.mintAccount,
      },
    });
    return {
      miner: this.minerKey,
      wrapper: this,
      tx: new TransactionEnvelope(this.provider, [instruction]),
    };
  }

  /**
   * Generates stake accounts for the user.
   * @returns
   */
  get userStakeAccounts(): MineUserStakeAccounts {
    const authority = this.authority;
    const miner = this.minerKey;
    const minerVault = this.tokenVaultKey;
    return {
      authority,
      miner,
      quarry: this.quarry.key,
      tokenAccount: this.stakedTokenATA,
      tokenProgram: TOKEN_PROGRAM_ID,
      rewarder: this.quarry.quarryData.rewarderKey,
      clock: SYSVAR_CLOCK_PUBKEY,
      minerVault,
    };
  }

  private performStakeAction(
    amount: TokenAmount,
    action: "stakeTokens" | "withdrawTokens"
  ): TransactionEnvelope {
    const instruction = this.program.instruction[action](amount.toU64(), {
      accounts: this.userStakeAccounts,
    });
    return new TransactionEnvelope(this.provider, [instruction]);
  }

  private async getOrCreateStakedAssociatedTokenAccountInternal() {
    return await getOrCreateATA({
      provider: this.provider,
      mint: this.quarry.token.mintAccount,
      owner: this.authority,
    });
  }

  /**
   * Creates the ATA of the user's staked token if it doesn't exist.
   */
  public async createATAIfNotExists(): Promise<TransactionEnvelope | null> {
    const { instruction } =
      await this.getOrCreateStakedAssociatedTokenAccountInternal();
    if (!instruction) {
      return null;
    }
    return new TransactionEnvelope(this.provider, [instruction]);
  }

  /**
   * Stakes the current wallet's tokens into the pool.
   * @param amount
   * @returns
   */
  public stake(amount: TokenAmount): TransactionEnvelope {
    return this.performStakeAction(amount, "stakeTokens");
  }

  /**
   * Withdraws the current wallet's tokens from the pool.
   * @param amount
   * @returns
   */
  public withdraw(amount: TokenAmount): TransactionEnvelope {
    return this.performStakeAction(amount, "withdrawTokens");
  }

  /**
   * Fetches the data associated with the miner.
   * @returns
   */
  public async fetchData(): Promise<MinerData> {
    return await this.program.account.miner.fetch(this.minerKey);
  }

  /**
   * Claims an amount of tokens.
   * @returns
   */
  public async claim(): Promise<TransactionEnvelope> {
    const instructions: TransactionInstruction[] = [];
    const { address: rewardsTokenAccount, instruction: ataInstruction } =
      await getOrCreateATA({
        provider: this.provider,
        mint: this.quarry.rewarderData.rewardsTokenMint,
        owner: this.authority,
      });
    if (ataInstruction) {
      instructions.push(ataInstruction);
    }

    const [minter] = await findMinterAddress(
      this.quarry.rewarderData.mintWrapper,
      this.quarry.quarryData.rewarderKey,
      this.sdk.mintWrapper.program.programId
    );

    const ix = this.quarry.program.instruction.claimRewards({
      accounts: {
        mintWrapper: this.quarry.rewarderData.mintWrapper,
        mintWrapperProgram: this.quarry.rewarderData.mintWrapperProgram,
        minter,
        rewardsTokenMint: this.quarry.rewarderData.rewardsTokenMint,
        rewardsTokenAccount,
        stake: this.userStakeAccounts,
        claimFeeTokenAccount: this.quarry.rewarderData.claimFeeTokenAccount,
      },
    });
    instructions.push(ix);

    return this.sdk.newTx(instructions);
  }
}
