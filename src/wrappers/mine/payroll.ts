import { MAX_U64 } from "@saberhq/token-utils";
import BN from "bn.js";

export const ZERO = new BN(0);
export const BASE_TEN = new BN(10);

export const MAX_U64_BN = new BN(MAX_U64.toString());

export const SECONDS_PER_YEAR = new BN(365 * 86_400);

export class Payroll {
  constructor(
    readonly famineTs: BN,
    readonly lastCheckpointTs: BN,
    readonly annualRewardsRate: BN,
    readonly rewardsPerTokenStored: BN,
    readonly totalTokensDeposited: BN
  ) {}

  /**
   * Calculates the amount of tokens that this user can receive.
   * @param currentTs
   * @returns
   */
  calculateRewardPerToken(currentTs: BN): BN {
    if (this.totalTokensDeposited.isZero()) {
      return this.rewardsPerTokenStored;
    }

    const lastTimeRewardsApplicable = BN.min(currentTs, this.famineTs);
    const timeWorked = BN.max(
      ZERO,
      lastTimeRewardsApplicable.sub(this.lastCheckpointTs)
    );
    const reward = timeWorked
      .mul(MAX_U64_BN)
      .mul(this.annualRewardsRate)
      .div(SECONDS_PER_YEAR)
      .div(this.totalTokensDeposited);
    return this.rewardsPerTokenStored.add(reward);
  }

  /**
   * Calculates the amount of tokens that this user can claim.
   * @param currentTs
   * @param tokensDeposited
   * @param rewardsPerTokenPaid
   * @param rewardsEarned
   * @returns
   */
  calculateRewardsEarned(
    currentTs: BN,
    tokensDeposited: BN,
    rewardsPerTokenPaid: BN,
    rewardsEarned: BN
  ): BN {
    const netNewRewards =
      this.calculateRewardPerToken(currentTs).sub(rewardsPerTokenPaid);
    const earnedRewards = tokensDeposited.mul(netNewRewards).div(MAX_U64_BN);
    return earnedRewards.add(rewardsEarned);
  }
}
