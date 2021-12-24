import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { Token } from "@saberhq/token-utils";
import { getATAAddress, getOrCreateATA, u64 } from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";

import type {
  MineProgram,
  MinerData,
  QuarryData,
  RewarderData,
} from "../../programs/mine";
import type { QuarrySDK } from "../../sdk";
import { MinerWrapper } from "./miner";
import { Payroll } from "./payroll";
import { findMinerAddress } from "./pda";
import type { PendingMiner } from "./types";

export class QuarryWrapper {
  constructor(
    readonly sdk: QuarrySDK,
    /**
     * The token being staked.
     */
    readonly token: Token,
    /**
     * The data of the rewarder.
     */
    readonly rewarderData: RewarderData,
    /**
     * The data of the quarry.
     */
    readonly quarryData: QuarryData,
    /**
     * The key of the quarry.
     */
    readonly key: PublicKey
  ) {}

  /**
   * The program.
   */
  get program(): MineProgram {
    return this.sdk.programs.Mine;
  }

  /**
   * The provider.
   */
  get provider(): Provider {
    return this.sdk.provider;
  }

  /**
   * Loads a quarry
   * @returns
   */
  static async load({
    sdk,
    token,
    key,
  }: {
    sdk: QuarrySDK;
    /**
     * The quarry's key
     */
    key: PublicKey;
    /**
     * The token being staked.
     */
    token: Token;
  }): Promise<QuarryWrapper> {
    const program = sdk.programs.Mine;
    const quarryData = await program.account.quarry.fetch(key);
    const rewarderData = await program.account.rewarder.fetch(
      quarryData.rewarderKey
    );
    return new QuarryWrapper(sdk, token, rewarderData, quarryData, key);
  }

  /**
   * Get the computed rewards rate of the quarry.
   *
   * This is used for tests, so you probably don't want this.
   * You want quarryData.annualRewardsRate.
   *
   * @returns annualRewardsRate
   */
  computeAnnualRewardsRate(): u64 {
    const rewarder = this.rewarderData;
    const totalRewardsShares = rewarder.totalRewardsShares;
    if (totalRewardsShares.isZero()) {
      return new u64(0);
    }
    const numerator = rewarder.annualRewardsRate.mul(
      this.quarryData.rewardsShare
    );
    return numerator.div(totalRewardsShares);
  }

  /**
   * Get the public key of the miner assocaited with the authority account
   * @param authority who owns the miner
   * @returns miner public key
   */
  async getMinerAddress(authority: PublicKey): Promise<PublicKey> {
    const [key] = await findMinerAddress(
      this.key,
      authority,
      this.program.programId
    );
    return key;
  }

  /**
   * Get the miner data associated with the authority account
   * @param authority
   * @returns
   */
  async getMiner(authority: PublicKey): Promise<MinerData | null> {
    try {
      return await this.program.account.miner.fetch(
        await this.getMinerAddress(authority)
      );
    } catch (e) {
      return null;
    }
  }

  /**
   * Get the miner associated with the authority account
   * @param authority
   * @returns
   */
  async getMinerActions(
    authority: PublicKey = this.program.provider.wallet.publicKey
  ): Promise<MinerWrapper> {
    const miner = await this.getMinerAddress(authority);
    const stakedTokenATA = await getATAAddress({
      mint: this.quarryData.tokenMintKey,
      owner: authority,
    });
    const tokenVaultKey = await getATAAddress({
      mint: this.quarryData.tokenMintKey,
      owner: miner,
    });
    return this.createMinerWrapper(
      authority,
      miner,
      tokenVaultKey,
      stakedTokenATA
    );
  }

  /**
   * Creates a miner wrapper
   * @param authority
   * @param minerKey
   * @param tokenVaultKey
   * @param stakedTokenATA
   * @returns
   */
  createMinerWrapper(
    authority: PublicKey,
    minerKey: PublicKey,
    tokenVaultKey: PublicKey,
    stakedTokenATA: PublicKey
  ): MinerWrapper {
    return new MinerWrapper(
      this,
      authority,
      minerKey,
      tokenVaultKey,
      stakedTokenATA
    );
  }

  /**
   * Sets the rewards share of this mine.
   */
  setRewardsShare(share: u64): TransactionEnvelope {
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.setRewardsShare(share, {
        accounts: {
          auth: {
            authority: this.provider.wallet.publicKey,
            rewarder: this.quarryData.rewarderKey,
          },
          quarry: this.key,
        },
      }),
    ]);
  }

  /**
   * Sets the famine timestampe for this mine.
   */
  setFamine(famineTs: u64): TransactionEnvelope {
    return new TransactionEnvelope(this.provider, [
      this.program.instruction.setFamine(famineTs, {
        accounts: {
          auth: {
            authority: this.provider.wallet.publicKey,
            rewarder: this.quarryData.rewarderKey,
          },
          quarry: this.key,
        },
      }),
    ]);
  }

  /**
   * Creates the miner of the provided wallet.
   */
  async createMiner({
    authority = this.program.provider.wallet.publicKey,
  }: {
    authority?: PublicKey;
  } = {}): Promise<PendingMiner> {
    const [miner, bump] = await findMinerAddress(
      this.key,
      authority,
      this.program.programId
    );
    const { address: minerVault, instruction: createATATX } =
      await getOrCreateATA({
        provider: this.provider,
        mint: this.quarryData.tokenMintKey,
        owner: miner,
      });
    const stakedTokenATA = await getATAAddress({
      mint: this.quarryData.tokenMintKey,
      owner: authority,
    });
    const wrapper = this.createMinerWrapper(
      authority,
      miner,
      minerVault,
      stakedTokenATA
    );
    const result = wrapper.initialize(bump);
    if (createATATX) {
      result.tx.instructions.unshift(createATATX);
    }
    return result;
  }

  /**
   * Payroll helper
   */
  get payroll(): Payroll {
    const data = this.quarryData;
    return new Payroll(
      data.famineTs,
      data.lastUpdateTs,
      data.annualRewardsRate,
      data.rewardsPerTokenStored,
      data.totalTokensDeposited
    );
  }
}
