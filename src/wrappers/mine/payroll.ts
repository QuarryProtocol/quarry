import BN from "bn.js";

export const ZERO = new BN(0);
export const BASE_TEN = new BN(10);

export class Payroll {
  constructor(
    public readonly famineTs: BN,
    public readonly lastCheckpointTs: BN,
    public readonly rewardsRatePerSecond: BN,
    public readonly rewardsPerTokenStored: BN,
    public readonly tokenDecimals: BN,
    public readonly totalTokensDeposited: BN
  ) {}

  /**
   * Calculates the amount of tokens that this user can receive.
   * @param current_ts
   * @returns
   */
  public calculateRewardPerToken(current_ts: BN): BN {
    if (this.totalTokensDeposited.isZero()) {
      return this.rewardsPerTokenStored;
    }

    const lastTimeRewardsApplicable = BN.min(current_ts, this.famineTs);
    const timeWorked = BN.max(
      ZERO,
      lastTimeRewardsApplicable.sub(this.lastCheckpointTs)
    );
    const reward = timeWorked.mul(this.rewardsRatePerSecond);
    const preciseReward = reward
      .mul(this.decimalPrecision())
      .div(this.totalTokensDeposited);

    return this.rewardsPerTokenStored.add(preciseReward);
  }

  /**
   * Calculates the amount of tokens that this user can claim.
   * @param currentTs
   * @param tokensDeposited
   * @param rewardsPerTokenPaid
   * @param rewardsEarned
   * @returns
   */
  public calculateRewardsEarned(
    currentTs: BN,
    tokensDeposited: BN,
    rewardsPerTokenPaid: BN,
    rewardsEarned: BN
  ): BN {
    const netNewRewards =
      this.calculateRewardPerToken(currentTs).sub(rewardsPerTokenPaid);
    const earnedRewards = tokensDeposited
      .mul(netNewRewards)
      .div(this.decimalPrecision());
    return earnedRewards.add(rewardsEarned);
  }

  private decimalPrecision(): BN {
    return BASE_TEN.pow(this.tokenDecimals);
  }
}
