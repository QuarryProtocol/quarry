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
import type { PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../..";
import type {
  MergeMinerData,
  MergePoolData,
  QuarryMergeMineProgram,
  QuarryStakeAccounts,
} from "../../programs";
import { findMinerAddress, findMinterAddress, findQuarryAddress } from "..";
import type { MergeMine } from "./quarryMergeMine";

export class MergeMiner {
  constructor(
    readonly mergeMine: MergeMine,
    readonly pool: {
      key: PublicKey;
      data: MergePoolData;
    },
    readonly mm: {
      key: PublicKey;
      data: MergeMinerData;
    }
  ) {}

  get provider(): Provider {
    return this.mergeMine.provider;
  }

  get program(): QuarryMergeMineProgram {
    return this.mergeMine.programs.MergeMine;
  }

  get primaryMint(): PublicKey {
    return this.pool.data.primaryMint;
  }

  get replicaMint(): PublicKey {
    return this.pool.data.replicaMint;
  }

  /**
   * Deposit primary tokens into the merge miner.
   * @param amount
   * @returns
   */
  async deposit({
    amount,
    rewarder,
  }: {
    amount: TokenAmount;
    rewarder: PublicKey;
  }): Promise<TransactionEnvelope> {
    const owner = this.provider.wallet.publicKey;
    const { address: ata, instruction } = await getOrCreateATA({
      provider: this.provider,
      mint: this.primaryMint,
      owner,
    });
    if (instruction) {
      throw new Error("User has no tokens to deposit");
    }
    const mmPrimaryTokenAccount = await getATAAddress({
      mint: this.primaryMint,
      owner: this.mm.key,
    });

    return new TransactionEnvelope(this.provider, [
      SPLToken.createTransferInstruction(
        TOKEN_PROGRAM_ID,
        ata,
        mmPrimaryTokenAccount,
        owner,
        [],
        amount.toU64()
      ),
    ]).combine(await this.stakePrimaryMiner(rewarder));
  }

  /**
   * Deposits tokens into the primary quarry.
   * (Not recommended-- you probably want {@link MergeMiner#deposit}.)
   * @returns
   */
  async stakePrimaryMiner(rewarder: PublicKey): Promise<TransactionEnvelope> {
    const stake = await this.getPrimaryStakeAccounts(rewarder);
    const mmPrimaryTokenAccount = await getATAAddress({
      mint: this.primaryMint,
      owner: this.mm.key,
    });
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.stakePrimaryMiner({
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
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
  async stakeReplicaMiner(rewarder: PublicKey): Promise<TransactionEnvelope> {
    const stake = await this.getReplicaStakeAccounts(rewarder);
    const [quarry] = await findQuarryAddress(rewarder, this.replicaMint);
    const [miner, minerBump] = await findMinerAddress(quarry, this.mm.key);

    const mmReplicaMintTokenAccount = await getOrCreateATA({
      provider: this.provider,
      mint: this.replicaMint,
      owner: this.mm.key,
    });
    const txEnv = new TransactionEnvelope(this.provider, [
      this.program.instruction.stakeReplicaMiner({
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
          replicaMint: this.replicaMint,
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
        mint: this.replicaMint,
        owner: miner,
      });
      txEnv.instructions.unshift(
        this.program.instruction.initMiner(minerBump, {
          accounts: {
            pool: this.pool.key,
            mm: this.mm.key,
            miner,
            quarry,
            rewarder,
            tokenMint: this.replicaMint,
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
      console.log("HERE");
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
  }: {
    amount: TokenAmount;
    rewarder: PublicKey;
  }): Promise<TransactionEnvelope> {
    const withdrawPrimary = await this.unstakePrimaryMiner(rewarder, amount);
    const withdrawPrimaryFromMM = await this.withdrawTokens(
      amount.token.mintAccount
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
  async unstakeAllReplica(rewarder: PublicKey): Promise<TransactionEnvelope> {
    const stake = await this.getReplicaStakeAccounts(rewarder);
    const replicaMintTokenAccount = await getATAAddress({
      mint: this.replicaMint,
      owner: this.mm.key,
    });
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.unstakeAllReplicaMiner({
        accounts: {
          mmOwner: this.provider.wallet.publicKey,
          replicaMint: this.replicaMint,
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
    amount: TokenAmount
  ): Promise<TransactionEnvelope> {
    const stake = await this.getPrimaryStakeAccounts(rewarder);
    const mmPrimaryTokenAccount = await getATAAddress({
      mint: this.primaryMint,
      owner: this.mm.key,
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
   * Withdraws unstaked primary tokens from the merge miner.
   * (Not recommended-- you probably want {@link MergeMiner#withdraw}.)
   * @returns
   */
  async withdrawPrimaryFromMM(): Promise<TransactionEnvelope> {
    const owner = this.provider.wallet.publicKey;
    const mmPrimaryAccount = await getATAAddress({
      mint: this.primaryMint,
      owner: this.mm.key,
    });
    const ownerPrimaryATA = await getOrCreateATA({
      provider: this.provider,
      mint: this.primaryMint,
      owner,
    });
    const withdrawPrimaryFromMMIx = this.program.instruction.withdrawTokens({
      accounts: {
        owner,
        pool: this.pool.key,
        mm: this.mm.key,
        mmTokenAccount: mmPrimaryAccount,
        withdrawMint: this.primaryMint,
        tokenDestination: ownerPrimaryATA.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    return new TransactionEnvelope(this.provider, [
      ...(ownerPrimaryATA.instruction ? [ownerPrimaryATA.instruction] : []),
      withdrawPrimaryFromMMIx,
    ]);
  }

  /**
   * Withdraws a specific mint from the merge miner.
   * @param withdrawMint
   * @returns
   */
  async withdrawTokens(withdrawMint: PublicKey): Promise<TransactionEnvelope> {
    const owner = this.provider.wallet.publicKey;
    const mmPrimaryAccount = await getATAAddress({
      mint: withdrawMint,
      owner: this.mm.key,
    });
    const ownerPrimaryATA = await getATAAddress({
      mint: withdrawMint,
      owner,
    });
    const withdrawTokensIX = this.program.instruction.withdrawTokens({
      accounts: {
        owner,
        pool: this.pool.key,
        mm: this.mm.key,
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
  async claimPrimaryRewards(rewarder: PublicKey): Promise<TransactionEnvelope> {
    return await this.claimRewardsCommon(
      this.primaryMint,
      await this.getPrimaryStakeAccounts(rewarder)
    );
  }

  /**
   * Claims rewards for a replica account.
   * @returns
   */
  async claimReplicaRewards(rewarder: PublicKey): Promise<TransactionEnvelope> {
    return await this.claimRewardsCommon(
      this.replicaMint,
      await this.getReplicaStakeAccounts(rewarder)
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
    const rewarderKey = stake.rewarder;
    const rewarder =
      await this.mergeMine.sdk.programs.Mine.account.rewarder.fetch(
        rewarderKey
      );
    const [minter] = await findMinterAddress(
      rewarder.mintWrapper,
      rewarderKey,
      this.mergeMine.sdk.programs.MintWrapper.programId
    );

    const mm = this.mm.key;
    const withdrawMint = rewarder.rewardsTokenMint;
    const mmATAs = await getOrCreateATAs({
      provider: this.provider,
      mints: {
        quarry: quarryMint,
        rewards: withdrawMint,
      },
      owner: mm,
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
      owner: this.pool.key,
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
    ]).combine(await this.withdrawTokens(withdrawMint));
  }

  async getReplicaStakeAccounts(
    rewarder: PublicKey
  ): Promise<QuarryStakeAccounts> {
    const [quarry] = await findQuarryAddress(rewarder, this.replicaMint);
    const [miner] = await findMinerAddress(quarry, this.mm.key);
    const minerVault = await getATAAddress({
      mint: this.replicaMint,
      owner: miner,
    });
    return {
      ...this.commonStakeAccounts,
      rewarder,
      quarry,
      miner,
      minerVault,
    };
  }

  async getPrimaryStakeAccounts(
    rewarder: PublicKey
  ): Promise<QuarryStakeAccounts> {
    const [quarry] = await findQuarryAddress(rewarder, this.primaryMint);
    const [miner] = await findMinerAddress(quarry, this.mm.key);
    const minerVault = await getATAAddress({
      mint: this.primaryMint,
      owner: miner,
    });
    return {
      ...this.commonStakeAccounts,
      rewarder,
      quarry,
      miner,
      minerVault,
      unusedAccount: Keypair.generate().publicKey,
    };
  }

  get commonStakeAccounts(): Pick<
    QuarryStakeAccounts,
    "pool" | "mm" | "tokenProgram" | "mineProgram" | "unusedAccount"
  > {
    return {
      pool: this.mm.data.pool,
      mm: this.mm.key,
      tokenProgram: TOKEN_PROGRAM_ID,
      mineProgram: this.mergeMine.sdk.mine.program.programId,
      unusedAccount: Keypair.generate().publicKey,
    };
  }
}
