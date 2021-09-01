//! Rewarder utilities.

use anchor_lang::prelude::*;
use anchor_lang::require;
use num_traits::ToPrimitive;
use vipers::unwrap_int;

use crate::Rewarder;

impl Rewarder {
    /// Computes the amount of rewards a [crate::Quarry] should receive, annualized.
    /// This should be run only after `total_rewards_shares` has been set.
    /// Do not call this directly. Use `compute_quarry_annual_rewards_rate`.
    fn compute_quarry_annual_rewards_rate_unsafe(&self, quarry_rewards_share: u64) -> Option<u64> {
        (self.annual_rewards_rate as u128)
            .checked_mul(quarry_rewards_share as u128)?
            .checked_div(self.total_rewards_shares as u128)?
            .to_u64()
    }

    /// Computes the amount of rewards a [crate::Quarry] should receive, annualized.
    /// This should be run only after `total_rewards_shares` has been set.
    pub fn compute_quarry_annual_rewards_rate(
        &self,
        quarry_rewards_share: u64,
    ) -> Result<u64, ProgramError> {
        require!(
            quarry_rewards_share <= self.total_rewards_shares,
            InvalidRewardsShare
        );

        // no rewards if:
        if self.total_rewards_shares == 0 // no shares
            || self.annual_rewards_rate == 0 // rewards rate is zero
            || quarry_rewards_share == 0
        // quarry has no share
        {
            return Ok(0);
        }

        let rate: u64 =
            unwrap_int!(self.compute_quarry_annual_rewards_rate_unsafe(quarry_rewards_share));

        Ok(rate)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand::thread_rng;
    use std::vec::Vec;
    use vipers::program_err;

    use crate::MAX_ANNUAL_REWARDS_RATE;

    const DEFAULT_ANNUAL_REWARDS_RATE: u64 = 100_000_000_000_000_000;

    fn add_quarry(l: &mut Rewarder, quarry_share: u64) {
        l.total_rewards_shares += quarry_share;
    }

    #[test]
    fn test_compute_quarry_annual_rewards_rate() {
        let mut rewarder = Rewarder {
            annual_rewards_rate: DEFAULT_ANNUAL_REWARDS_RATE,
            ..Default::default()
        };

        let invalid: Result<u64, ProgramError> = program_err!(InvalidRewardsShare);

        // invalid because there are no shares
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(DEFAULT_ANNUAL_REWARDS_RATE),
            invalid
        );

        rewarder.total_rewards_shares = 1_000_000_000_000;
        let tokens_per_share = DEFAULT_ANNUAL_REWARDS_RATE / rewarder.total_rewards_shares;

        assert_eq!(rewarder.compute_quarry_annual_rewards_rate(0), Ok(0));
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(1),
            Ok(tokens_per_share)
        );
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(10),
            Ok(10 * tokens_per_share)
        );
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(100),
            Ok(100 * tokens_per_share)
        );
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(1_000),
            Ok(1_000 * tokens_per_share)
        );

        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(10_000),
            Ok(10_000 * tokens_per_share)
        );
        assert_eq!(
            rewarder.compute_quarry_annual_rewards_rate(100_000),
            Ok(100_000 * tokens_per_share)
        );
    }

    #[test]
    fn test_compute_quarry_rewards_rate_with_multiple_quarries_fixed() {
        let rewarder = &mut Rewarder::default();
        rewarder.annual_rewards_rate = DEFAULT_ANNUAL_REWARDS_RATE;
        rewarder.num_quarries = 1_000;

        let mut rng = thread_rng();
        let mut quarry_rewards_shares: Vec<u64> = Vec::new();
        for _ in 0..rewarder.num_quarries {
            let quarry_rewards_share: u32 = rng.gen_range(1..rewarder.annual_rewards_rate as u32);
            add_quarry(rewarder, quarry_rewards_share as u64);
            quarry_rewards_shares.push(quarry_rewards_share.into());
        }

        let mut total_rewards_per_day: u64 = 0;
        for i in 0..rewarder.num_quarries {
            total_rewards_per_day += rewarder
                .compute_quarry_annual_rewards_rate(quarry_rewards_shares[i as usize])
                .unwrap();
        }
        let diff = rewarder.annual_rewards_rate - total_rewards_per_day;

        const MAX_EPSILON: u64 = 30;
        let num_quarries = rewarder.num_quarries as u64;
        let epsilon: u64 = if diff > num_quarries / 2 {
            diff - num_quarries / 2
        } else {
            num_quarries / 2 - diff
        };
        assert!(
            epsilon <= MAX_EPSILON,
            "diff: {}, num_quarries / 2: {}, epsilon: {}",
            diff,
            num_quarries / 2,
            epsilon
        );
    }

    proptest! {
        #[test]
        fn test_compute_rewards_rate_when_total_rewards_shares_is_zero(
            num_quarries in 0..u16::MAX,
            annual_rewards_rate in 0..u64::MAX,
            quarry_rewards_share in 0..u64::MAX,
        ) {
            let rewarder = Rewarder {
                bump: 254,
                num_quarries,
                annual_rewards_rate,
                ..Default::default()
            };
            assert_eq!(rewarder.compute_quarry_annual_rewards_rate(quarry_rewards_share), program_err!(InvalidRewardsShare));
            assert_eq!(rewarder.compute_quarry_annual_rewards_rate(0), Ok(0));
        }
    }

    proptest! {
        #[test]
        fn test_compute_quarry_rewards_rate_with_multiple_quarries(
            annual_rewards_rate in 0..=MAX_ANNUAL_REWARDS_RATE,
            num_quarries in 0..=u16::MAX,
            total_rewards_shares in 0..=u64::MAX
        ) {
            let rewarder = &mut Rewarder::default();
            rewarder.annual_rewards_rate = annual_rewards_rate;

            let mut rng = thread_rng();
            let mut quarry_rewards_shares: Vec<u64> = Vec::new();

            let mut total_rewards_shares_remaining = total_rewards_shares;
            // add all quarries
            for _ in 0..(num_quarries - 1) {
                let quarry_rewards_share: u64 = rng.gen_range(0..(total_rewards_shares_remaining / (num_quarries as u64)));
                add_quarry(rewarder, quarry_rewards_share);
                quarry_rewards_shares.push(quarry_rewards_share);
                total_rewards_shares_remaining -= quarry_rewards_share;
            }
            // last quarry gets the remaining shares
            add_quarry(rewarder, total_rewards_shares_remaining);
            quarry_rewards_shares.push(total_rewards_shares_remaining);

            let mut total_rewards_per_year: u64 = 0;
            for i in 0..num_quarries {
                total_rewards_per_year += rewarder.compute_quarry_annual_rewards_rate(quarry_rewards_shares[i as usize]).unwrap();
            }
            let diff = rewarder.annual_rewards_rate - total_rewards_per_year;

            // the maximum discrepancy should be the number of quarries
            // each of their rewards can be reduced by 1
            let max_diff: u64 = num_quarries.into();
            assert!(diff <= max_diff, "diff: {}, num quarries: {}", diff, num_quarries);
        }
    }
}
