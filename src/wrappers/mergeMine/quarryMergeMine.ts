import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import {
  deserializeMint,
  getOrCreateATA,
  getOrCreateATAs,
  Token,
  TOKEN_PROGRAM_ID,
} from "@saberhq/token-utils";
import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

import type { Programs } from "../../constants";
import type {
  MergeMinerData,
  MergePoolData,
  QuarryMergeMineProgram,
} from "../../programs";
import type { QuarrySDK } from "../../sdk";
import { findMinerAddress, findQuarryAddress } from "../mine/pda";
import { MergeMiner } from "./mergeMiner";
import { MergePool } from "./mergePool";
import {
  findMergeMinerAddress,
  findPoolAddress,
  findReplicaMintAddress,
} from "./pda";

export class MergeMine {
  constructor(readonly sdk: QuarrySDK) {}

  get programs(): Programs {
    return this.sdk.programs;
  }

  get program(): QuarryMergeMineProgram {
    return this.programs.MergeMine;
  }

  get provider(): Provider {
    return this.sdk.provider;
  }

  /**
   * Creates a new pool.
   * @returns
   */
  async newPool({
    primaryMint,
    payer = this.provider.wallet.publicKey,
  }: {
    /**
     * Primary mint.
     */
    primaryMint: PublicKey;
    payer?: PublicKey;
  }): Promise<{
    key: PublicKey;
    tx: TransactionEnvelope;
    replicaToken: Token;
  }> {
    const [primaryMintRaw] = await Promise.all([
      this.provider.getAccountInfo(primaryMint),
    ]);
    if (!primaryMintRaw) {
      throw new Error(`Could not find primary mint: ${primaryMint.toString()}`);
    }

    const parsedMint = deserializeMint(primaryMintRaw.accountInfo.data);

    const [pool, bump] = await findPoolAddress({
      programId: this.program.programId,
      primaryMint,
    });
    const [replicaMint, mintBump] = await findReplicaMintAddress({
      programId: this.program.programId,
      primaryMint,
    });

    const newPoolIx = this.program.instruction.newPool(bump, mintBump, {
      accounts: {
        pool,
        payer,
        primaryMint,
        replicaMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      },
    });

    const createPool = new TransactionEnvelope(this.provider, [newPoolIx]);

    return {
      key: pool,
      tx: createPool,
      replicaToken: Token.fromMint(replicaMint, parsedMint.decimals),
    };
  }

  async fetchMergePoolData(
    key: PublicKey
  ): Promise<{ key: PublicKey; data: MergePoolData }> {
    return {
      key,
      data: await this.program.account.mergePool.fetch(key),
    };
  }

  async fetchMergeMinerData(
    key: PublicKey
  ): Promise<{ key: PublicKey; data: MergeMinerData }> {
    return {
      key,
      data: await this.program.account.mergeMiner.fetch(key),
    };
  }

  async findPoolAddress({
    primaryMint,
  }: {
    primaryMint: PublicKey;
  }): Promise<PublicKey> {
    const [pool] = await findPoolAddress({
      programId: this.program.programId,
      primaryMint,
    });
    return pool;
  }

  async findMergeMinerAddress({
    owner = this.provider.wallet.publicKey,
    pool,
  }: {
    owner?: PublicKey;
    pool: PublicKey;
  }): Promise<PublicKey> {
    const [mm] = await findMergeMinerAddress({
      programId: this.program.programId,
      pool,
      owner,
    });
    return mm;
  }

  /**
   * Creates a new MM.
   * @param param0
   * @returns
   */
  async newMM({
    owner = this.provider.wallet.publicKey,
    payer = this.provider.wallet.publicKey,
    pool: {
      key: poolKey,
      data: { primaryMint },
    },
    rewarder,
    rewardsMint,
  }: {
    owner?: PublicKey;
    payer?: PublicKey;
    pool: {
      key: PublicKey;
      data: Pick<MergePoolData, "primaryMint">;
    };
    /**
     * Rewarder to deposit into.
     */
    rewarder: PublicKey;
    /**
     * Mint received as rewards from the initial rewarder.
     */
    rewardsMint: PublicKey;
  }): Promise<{ key: PublicKey; tx: TransactionEnvelope | null }> {
    const [mm, bump] = await findMergeMinerAddress({
      programId: this.program.programId,
      pool: poolKey,
      owner,
    });

    // mm ATAs
    const { instructions } = await getOrCreateATAs({
      provider: this.provider,
      mints: {
        rewards: rewardsMint,
        primary: primaryMint,
      },
      owner: mm,
    });

    const allInstructions = [...instructions];
    const mergeMinerAccountInfo =
      await this.sdk.provider.connection.getAccountInfo(mm);
    if (!mergeMinerAccountInfo) {
      allInstructions.push(
        this.program.instruction.initMergeMiner(bump, {
          accounts: {
            pool: poolKey,
            owner,
            mm,
            payer,
            systemProgram: SystemProgram.programId,
          },
        })
      );
    }

    const { ixs: initPrimaryIxs } = await this.getOrCreatePrimary({
      mint: primaryMint,
      pool: poolKey,
      mm,
      payer,
      rewarder,
    });
    allInstructions.push(...initPrimaryIxs);

    return {
      key: mm,
      tx: allInstructions.length
        ? new TransactionEnvelope(this.provider, allInstructions)
        : null,
    };
  }

  async getOrCreatePrimary({
    mint,
    pool,
    mm,
    payer = this.provider.wallet.publicKey,
    rewarder,
  }: {
    mint: PublicKey;
    pool: PublicKey;
    mm: PublicKey;
    payer?: PublicKey;
    rewarder: PublicKey;
  }): Promise<{
    miner: PublicKey;
    ixs: TransactionInstruction[];
  }> {
    const [quarryKey] = await findQuarryAddress(rewarder, mint);
    const [minerKey, minerBump] = await findMinerAddress(quarryKey, mm);

    const ixs: TransactionInstruction[] = [];
    const minerAccountInfo = await this.sdk.provider.connection.getAccountInfo(
      minerKey
    );
    if (minerAccountInfo) {
      return { miner: minerKey, ixs };
    }

    const minerATA = await getOrCreateATA({
      provider: this.provider,
      mint,
      owner: minerKey,
    });
    if (minerATA.instruction) {
      ixs.push(minerATA.instruction);
    }
    ixs.push(
      this.program.instruction.initMiner(minerBump, {
        accounts: {
          mineProgram: this.sdk.mine.program.programId,
          pool,
          mm,
          systemProgram: SystemProgram.programId,
          payer,
          tokenProgram: TOKEN_PROGRAM_ID,
          rewarder,
          miner: minerKey,
          quarry: quarryKey,
          tokenMint: mint,
          minerVault: minerATA.address,
        },
      })
    );

    return {
      miner: minerKey,
      ixs,
    };
  }

  async initMiner({
    mint,
    pool,
    mm,
    payer = this.provider.wallet.publicKey,
    rewarder,
  }: {
    mint: PublicKey;
    pool: PublicKey;
    mm: PublicKey;
    payer?: PublicKey;
    rewarder: PublicKey;
  }): Promise<{ tx: TransactionEnvelope; miner: PublicKey }> {
    const [quarryKey] = await findQuarryAddress(rewarder, mint);
    const [minerKey, minerBump] = await findMinerAddress(quarryKey, mm);
    const minerATA = await getOrCreateATA({
      provider: this.provider,
      mint,
      owner: minerKey,
    });
    const initMinerIX = this.program.instruction.initMiner(minerBump, {
      accounts: {
        mineProgram: this.sdk.mine.program.programId,
        pool,
        mm,
        systemProgram: SystemProgram.programId,
        payer,
        tokenProgram: TOKEN_PROGRAM_ID,
        rewarder,
        miner: minerKey,
        quarry: quarryKey,
        tokenMint: mint,
        minerVault: minerATA.address,
      },
    });
    return {
      tx: new TransactionEnvelope(this.provider, [
        ...(minerATA.instruction ? [minerATA.instruction] : []),
        initMinerIX,
      ]),
      miner: minerKey,
    };
  }

  /**
   * Loads a mm.
   * @returns
   */
  async loadMM({ mmKey }: { mmKey: PublicKey }): Promise<MergeMiner> {
    const mm = await this.fetchMergeMinerData(mmKey);
    const pool = await this.fetchMergePoolData(mm.data.pool);
    return new MergeMiner(this, pool, mm);
  }

  /**
   * Loads a mp.
   * @returns
   */
  loadMP({ mpKey }: { mpKey: PublicKey }): MergePool {
    return new MergePool(this, mpKey);
  }
}
