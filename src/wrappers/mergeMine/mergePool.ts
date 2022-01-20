import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { TokenAmount } from "@saberhq/token-utils";
import {
  getATAAddress,
  getOrCreateATA,
  getOrCreateATAs,
  SPLToken,
  TOKEN_PROGRAM_ID,
} from "@saberhq/token-utils";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { Keypair, SystemProgram } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";
import type {
  MergePoolData,
  QuarryMergeMineProgram,
  QuarryStakeAccounts,
} from "../../programs";
import { findMinterAddress } from "..";
import { findMinerAddress, findQuarryAddress } from "../mine/pda";
import { findMergeMinerAddress } from "./pda";
import type { MergeMine } from "./quarryMergeMine";

export class MergePool {
  private _data: MergePoolData | null = null;

  constructor(readonly mergeMine: MergeMine, readonly key: PublicKey) {}

  async reloadData(): Promise<MergePoolData> {
    this._data = await this.mergeMine.program.account.mergePool.fetch(this.key);
    return this._data;
  }

  async data(): Promise<MergePoolData> {
    if (this._data) {
      return this._data;
    }
    return await this.reloadData();
  }

  get provider(): Provider {
    return this.mergeMine.provider;
  }

  get program(): QuarryMergeMineProgram {
    return this.mergeMine.programs.MergeMine;
  }

  /**
   * Deposit primary tokens into the merge miner.
   * @param amount
   * @returns
   */
  async deposit({
    amount,
    rewarder,
    mmOwner = this.provider.wallet.publicKey,
  }: {
    amount: TokenAmount;
    rewarder: PublicKey;
    mmOwner?: PublicKey;
  }): Promise<TransactionEnvelope> {
    const poolData = await this.data();

    const { address: ata, instruction } = await getOrCreateATA({
      provider: this.provider,
      mint: poolData.primaryMint,
      owner: mmOwner,
    });
    if (instruction) {
      throw new Error("User has no tokens to deposit");
    }
    const [mmKey, bump] = await findMergeMinerAddress({
      pool: this.key,
      owner: mmOwner,
    });

    const mmAccount = await this.provider.getAccountInfo(mmKey);
    const { address: mmPrimaryTokenAccount, instruction: mmATAIx } =
      await getOrCreateATA({
        provider: this.provider,
        mint: poolData.primaryMint,
        owner: mmKey,
      });
    const allInstructions: TransactionInstruction[] = [];
    // Initialize mergeMiner if it does not exist
    if (!mmAccount) {
      allInstructions.push(
        this.program.instruction.initMergeMiner(bump, {
          accounts: {
            pool: this.key,
            owner: mmOwner,
            mm: mmKey,
            payer: this.provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          },
        })
      );
      if (mmATAIx) {
        allInstructions.push(mmATAIx);
      }
      const { ixs: initPrimaryIxs } = await this.mergeMine.getOrCreatePrimary({
        mint: poolData.primaryMint,
        pool: this.key,
        mm: mmKey,
        payer: this.provider.wallet.publicKey,
        rewarder,
      });
      allInstructions.push(...initPrimaryIxs);
    }

    allInstructions.push(
      SPLToken.createTransferInstruction(
        TOKEN_PROGRAM_ID,
        ata,
        mmPrimaryTokenAccount,
        mmOwner,
        [],
        amount.toU64()
      )
    );

    return new TransactionEnvelope(this.provider, allInstructions).combine(
      await this.stakePrimaryMiner(rewarder, mmKey)
    );
  }

  /**
   * Deposits tokens into the primary quarry.
   * (Not recommended-- you probably want {@link MergeMiner#deposit}.)
   * @returns
   */
  async stakePrimaryMiner(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const { provider } = this.mergeMine;
    const data = await this.data();

    const stake = await this.getPrimaryStakeAccounts(rewarder, mergeMiner);
    const mmPrimaryTokenAccount = await getATAAddress({
      mint: data.primaryMint,
      owner: mergeMiner,
    });
    return new TransactionEnvelope(provider, [
      this.mergeMine.program.instruction.stakePrimaryMiner({
        accounts: {
          mmOwner: provider.wallet.publicKey,
          mmPrimaryTokenAccount,
          stake,
        },
      }),
    ]);
  }

  /**
   * Stakes replica tokens into a miner.
   * @returns
   */
  async stakeReplicaMiner(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const poolData = await this.data();

    const stake = await this.getReplicaStakeAccounts(rewarder, mergeMiner);
    const [quarry] = await findQuarryAddress(rewarder, poolData.replicaMint);
    const [miner, minerBump] = await findMinerAddress(quarry, mergeMiner);

    const mmReplicaMintTokenAccount = await getOrCreateATA({
      provider: this.provider,
      mint: poolData.replicaMint,
      owner: mergeMiner,
    });
    const txEnv = new TransactionEnvelope(this.provider, [
      this.program.instruction.stakeReplicaMiner({
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
          replicaMint: poolData.replicaMint,
          replicaMintTokenAccount: mmReplicaMintTokenAccount.address,
          stake,
        },
      }),
    ]);
    if (mmReplicaMintTokenAccount.instruction) {
      txEnv.instructions.unshift(mmReplicaMintTokenAccount.instruction);
    }

    // initialize the miner if it does not exist
    if (!(await this.provider.getAccountInfo(miner))) {
      const minerReplicaMintTokenAccount = await getOrCreateATA({
        provider: this.provider,
        mint: poolData.replicaMint,
        owner: miner,
      });
      txEnv.instructions.unshift(
        this.program.instruction.initMiner(minerBump, {
          accounts: {
            pool: this.key,
            mm: mergeMiner,
            miner,
            quarry,
            rewarder,
            tokenMint: poolData.replicaMint,
            minerVault: minerReplicaMintTokenAccount.address,
            payer: this.provider.wallet.publicKey,
            mineProgram: QUARRY_ADDRESSES.Mine,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        })
      );
      if (minerReplicaMintTokenAccount.instruction) {
        txEnv.instructions.unshift(minerReplicaMintTokenAccount.instruction);
      }
    } else {
      console.error("HERE");
    }

    return txEnv;
  }

  /**
   * Withdraw staked tokens from a merge miner.
   * @param amount
   * @returns
   */
  async withdraw({
    amount,
    rewarder,
    mergeMiner,
  }: {
    amount: TokenAmount;
    rewarder: PublicKey;
    mergeMiner: PublicKey;
  }): Promise<TransactionEnvelope> {
    const withdrawPrimary = await this.unstakePrimaryMiner(
      rewarder,
      mergeMiner,
      amount
    );
    const withdrawPrimaryFromMM = await this.withdrawTokens(
      amount.token.mintAccount,
      mergeMiner
    );
    return TransactionEnvelope.combineAll(
      withdrawPrimary,
      withdrawPrimaryFromMM
    );
  }

  /**
   * Unstakes all replica tokens from a quarry.
   * You must call this function for each replica miner before unstaking the primary.
   * @returns
   */
  async unstakeAllReplica(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const poolData = await this.data();

    const stake = await this.getReplicaStakeAccounts(rewarder, mergeMiner);
    const replicaMintTokenAccount = await getATAAddress({
      mint: poolData.replicaMint,
      owner: mergeMiner,
    });
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.unstakeAllReplicaMiner({
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
          replicaMint: poolData.replicaMint,
          replicaMintTokenAccount,
          stake,
        },
      }),
    ]);
  }

  /**
   * Withdraws primary tokens from the quarry.
   * (Not recommended-- you probably want {@link MergeMiner#withdraw}.)
   * @returns
   */
  async unstakePrimaryMiner(
    rewarder: PublicKey,
    mergeMiner: PublicKey,
    amount: TokenAmount
  ): Promise<TransactionEnvelope> {
    const poolData = await this.data();

    const stake = await this.getPrimaryStakeAccounts(rewarder, mergeMiner);
    const mmPrimaryTokenAccount = await getATAAddress({
      mint: poolData.primaryMint,
      owner: mergeMiner,
    });
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.unstakePrimaryMiner(amount.toU64(), {
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
          mmPrimaryTokenAccount,
          stake,
        },
      }),
    ]);
  }

  /**
   * Withdraws a specific mint from the merge miner.
   * @param withdrawMint
   * @returns
   */
  async withdrawTokens(
    withdrawMint: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const owner = this.provider.wallet.publicKey;
    const mmPrimaryAccount = await getATAAddress({
      mint: withdrawMint,
      owner: mergeMiner,
    });
    const ownerPrimaryATA = await getATAAddress({
      mint: withdrawMint,
      owner,
    });
    const withdrawTokensIX = this.program.instruction.withdrawTokens({
      accounts: {
        owner,
        pool: this.key,
        mm: mergeMiner,
        mmTokenAccount: mmPrimaryAccount,
        tokenDestination: ownerPrimaryATA,
        tokenProgram: TOKEN_PROGRAM_ID,
        withdrawMint,
      },
    });
    return new TransactionEnvelope(this.provider, [withdrawTokensIX]);
  }

  /**
   * Claims rewards for a primary account.
   * @param rewarder
   * @returns
   */
  async claimPrimaryRewards(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const poolData = await this.data();
    return await this.claimRewardsCommon(
      poolData.primaryMint,
      await this.getPrimaryStakeAccounts(rewarder, mergeMiner)
    );
  }

  /**
   * Claims rewards for a replica account.
   * @returns
   */
  async claimReplicaRewards(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<TransactionEnvelope> {
    const poolData = await this.data();

    return await this.claimRewardsCommon(
      poolData.replicaMint,
      await this.getReplicaStakeAccounts(rewarder, mergeMiner)
    );
  }

  /**
   * Claims internal mining rewards.
   * @param amount
   * @returns
   */
  async claimRewardsCommon(
    quarryMint: PublicKey,
    stake: QuarryStakeAccounts,
    mmOwner: PublicKey = this.provider.wallet.publicKey
  ): Promise<TransactionEnvelope> {
    const rewarder =
      await this.mergeMine.sdk.programs.Mine.account.rewarder.fetch(
        stake.rewarder
      );
    const [minter] = await findMinterAddress(
      rewarder.mintWrapper,
      stake.rewarder,
      this.mergeMine.sdk.programs.MintWrapper.programId
    );

    const withdrawMint = rewarder.rewardsTokenMint;
    const mmATAs = await getOrCreateATAs({
      provider: this.provider,
      mints: {
        quarry: quarryMint,
        rewards: withdrawMint,
      },
      owner: stake.mm,
    });

    const ownerATAs = await getOrCreateATAs({
      provider: this.provider,
      mints: {
        rewards: withdrawMint,
      },
      owner: mmOwner,
    });
    const feeATA = await getOrCreateATA({
      provider: this.provider,
      mint: withdrawMint,
      owner: this.key,
    });

    return new TransactionEnvelope(this.provider, [
      ...mmATAs.instructions,
      ...ownerATAs.instructions,
      ...(feeATA.instruction ? [feeATA.instruction] : []),
      this.program.instruction.claimRewards({
        accounts: {
          mintWrapper: rewarder.mintWrapper,
          mintWrapperProgram: this.mergeMine.sdk.programs.MintWrapper.programId,
          minter,
          rewardsTokenMint: withdrawMint,
          rewardsTokenAccount: mmATAs.accounts.rewards,
          claimFeeTokenAccount: rewarder.claimFeeTokenAccount,
          stakeTokenAccount: mmATAs.accounts.quarry,
          stake,
        },
      }),
    ]).combine(await this.withdrawTokens(withdrawMint, stake.mm));
  }

  async getReplicaStakeAccounts(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<QuarryStakeAccounts> {
    const poolData = await this.data();

    const [quarry] = await findQuarryAddress(rewarder, poolData.replicaMint);
    const [miner] = await findMinerAddress(quarry, mergeMiner);
    const minerVault = await getATAAddress({
      mint: poolData.replicaMint,
      owner: miner,
    });
    return {
      mm: mergeMiner,
      rewarder,
      quarry,
      miner,
      minerVault,
      ...this.commonAccounts,
    };
  }

  async getPrimaryStakeAccounts(
    rewarder: PublicKey,
    mergeMiner: PublicKey
  ): Promise<QuarryStakeAccounts> {
    const poolData = await this.data();

    const [quarry] = await findQuarryAddress(rewarder, poolData.primaryMint);
    const [miner] = await findMinerAddress(quarry, mergeMiner);
    const minerVault = await getATAAddress({
      mint: poolData.primaryMint,
      owner: miner,
    });
    return {
      mm: mergeMiner,
      rewarder,
      quarry,
      miner,
      minerVault,
      ...this.commonAccounts,
      unusedAccount: Keypair.generate().publicKey,
    };
  }

  get commonAccounts(): Pick<
    QuarryStakeAccounts,
    "pool" | "tokenProgram" | "unusedAccount" | "mineProgram"
  > {
    return {
      pool: this.key,
      tokenProgram: TOKEN_PROGRAM_ID,
      mineProgram: this.mergeMine.programs.Mine.programId,
      unusedAccount: Keypair.generate().publicKey,
    };
  }
}
