import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { TokenAmount } from "@saberhq/token-utils";
import { getOrCreateATA, TOKEN_PROGRAM_ID } from "@saberhq/token-utils";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { Keypair, SystemProgram } from "@solana/web3.js";

import type { MineProgram, MinerData } from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { findMinterAddress } from "../mintWrapper/pda";
import type { QuarryWrapper } from "./quarry";
import type { PendingMiner } from "./types";

type MineUserStakeAccounts = Parameters<
  MineProgram["instruction"]["stakeTokens"]["accounts"]
>[0];

type MineUserClaimAccounts = Parameters<
  MineProgram["instruction"]["claimRewards"]["accounts"]
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
    readonly quarry: QuarryWrapper,
    readonly authority: PublicKey,
    readonly minerKey: PublicKey,
    readonly tokenVaultKey: PublicKey,
    readonly stakedTokenATA: PublicKey
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
  initialize(bump: number): PendingMiner {
    const instruction = this.program.instruction.createMiner(bump, {
      accounts: {
        authority: this.authority,
        miner: this.minerKey,
        quarry: this.quarry.key,
        systemProgram: SystemProgram.programId,
        payer: this.program.provider.wallet.publicKey,
        minerVault: this.tokenVaultKey,
        rewarder: this.quarry.quarryData.rewarderKey,
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
    const minerVault = this.tokenVaultKey;
    return {
      ...this.userClaimAccounts,
      tokenAccount: this.stakedTokenATA,
      minerVault,
    };
  }

  /**
   * Generates stake accounts for the user.
   * @returns
   */
  get userClaimAccounts(): MineUserClaimAccounts {
    const authority = this.authority;
    const miner = this.minerKey;
    const randomMut = Keypair.generate().publicKey;
    return {
      authority,
      miner,
      quarry: this.quarry.key,
      tokenProgram: TOKEN_PROGRAM_ID,
      rewarder: this.quarry.quarryData.rewarderKey,

      // dummies for backwards compatibility
      unusedMinerVault: randomMut,
      unusedTokenAccount: randomMut,
    };
  }

  private _performStakeAction(
    amount: TokenAmount,
    action: "stakeTokens" | "withdrawTokens"
  ): TransactionEnvelope {
    const instruction = this.program.instruction[action](amount.toU64(), {
      accounts: this.userStakeAccounts,
    });
    return new TransactionEnvelope(this.provider, [instruction]);
  }

  private async _getOrCreateStakedAssociatedTokenAccountInternal() {
    return await getOrCreateATA({
      provider: this.provider,
      mint: this.quarry.token.mintAccount,
      owner: this.authority,
    });
  }

  /**
   * Creates the ATA of the user's staked token if it doesn't exist.
   */
  async createATAIfNotExists(): Promise<TransactionEnvelope | null> {
    const { instruction } =
      await this._getOrCreateStakedAssociatedTokenAccountInternal();
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
  stake(amount: TokenAmount): TransactionEnvelope {
    return this._performStakeAction(amount, "stakeTokens");
  }

  /**
   * Withdraws the current wallet's tokens from the pool.
   * @param amount
   * @returns
   */
  withdraw(amount: TokenAmount): TransactionEnvelope {
    return this._performStakeAction(amount, "withdrawTokens");
  }

  /**
   * Fetches the data associated with the miner.
   * @returns
   */
  async fetchData(): Promise<MinerData> {
    return await this.program.account.miner.fetch(this.minerKey);
  }

  /**
   * Claims an amount of tokens.
   * @returns
   */
  async claim(): Promise<TransactionEnvelope> {
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
        minter,
        rewardsTokenMint: this.quarry.rewarderData.rewardsTokenMint,
        rewardsTokenAccount,
        stake: this.userClaimAccounts,
        mintWrapperProgram: this.sdk.programs.MintWrapper.programId,
        claimFeeTokenAccount: this.quarry.rewarderData.claimFeeTokenAccount,
      },
    });
    instructions.push(ix);

    return this.sdk.newTx(instructions);
  }
}
