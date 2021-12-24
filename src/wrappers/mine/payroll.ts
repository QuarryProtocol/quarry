import { MAX_U64 } from "@saberhq/token-utils";
import BN from "bn.js";

export const ZERO = new BN(0);
export const BASE_TEN = new BN(10);

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
   * @param current_ts
   * @returns
   */
  calculateRewardPerToken(current_ts: BN): BN {
    if (this.totalTokensDeposited.isZero()) {
      return this.rewardsPerTokenStored;
    }

    const lastTimeRewardsApplicable = BN.min(current_ts, this.famineTs);
    const timeWorked = BN.max(
      ZERO,
      lastTimeRewardsApplicable.sub(this.lastCheckpointTs)
    );
    const reward = timeWorked
      .mul(new BN(MAX_U64.toString()))
      .mul(this.annualRewardsRate)
      .div(new BN(365 * 86_400))
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
    const earnedRewards = tokensDeposited
      .mul(netNewRewards)
      .div(new BN(MAX_U64.toString()));
    return earnedRewards.add(rewardsEarned);
  }
}
