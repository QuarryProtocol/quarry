//! Rewarder utilities.
use crate::payroll::SECONDS_PER_DAY;
use crate::Rewarder;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::Key;
use vipers::program_err;

impl Rewarder {
    /// Enforces that the [Rewarder] owner is the caller.
    pub fn only_owner<'info>(&self, authority_account: &AccountInfo<'info>) -> ProgramResult {
        if !authority_account.is_signer || self.authority != authority_account.key() {
            return program_err!(Unauthorized);
        }
        Ok(())
    }

    /// Computes the rate of rewards of a [crate::Quarry] for a given quarry share.
    pub fn compute_quarry_annual_rewards_rate(&self, quarry_rewards_share: u64) -> u128 {
        if self.total_rewards_shares == 0 {
            0
        } else {
            self.annual_rewards_rate as u128 * quarry_rewards_share as u128
                / self.total_rewards_shares as u128
        }
    }

    /// Validate the quarry rewards share.
    pub fn validate_quarry_rewards_share(&self, quarry_rewards_share: u64) -> bool {
        if self.annual_rewards_rate == 0
            || self.total_rewards_shares == 0
            || quarry_rewards_share == 0
        {
            true
        } else {
            let daily_rate: u128 = self.compute_quarry_annual_rewards_rate(quarry_rewards_share);
            daily_rate / SECONDS_PER_DAY > 0
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::payroll::SECONDS_PER_DAY;

    use super::*;
    use proptest::prelude::*;
    use rand::thread_rng;
    use std::convert::TryFrom;
    use std::vec::Vec;

    const DEFAULT_ANNUAL_REWARDS_RATE: u64 = 71_428_571_428_600;
    const MAX_REWARDS_RATE: u64 = 1_000_000_000_000;
    const MAX_QUARRIES: u64 = 10_000;

    fn add_quarry(l: &mut Rewarder, quarry_share: u64) {
        l.total_rewards_shares += quarry_share;
    }

    #[test]
    fn test_validate_quarry_rewards_share() {
        let mut rewarder = Rewarder {
            annual_rewards_rate: DEFAULT_ANNUAL_REWARDS_RATE,
            ..Default::default()
        };
        assert!(rewarder.validate_quarry_rewards_share(DEFAULT_ANNUAL_REWARDS_RATE));
        rewarder.total_rewards_shares = 1_000_000_000_000;
        assert!(rewarder.validate_quarry_rewards_share(0));

        assert!(!rewarder.validate_quarry_rewards_share(1));
        assert!(!rewarder.validate_quarry_rewards_share(10));
        assert!(!rewarder.validate_quarry_rewards_share(100));
        assert!(!rewarder.validate_quarry_rewards_share(1000));
        assert!(rewarder.validate_quarry_rewards_share(10000));
        assert!(rewarder.validate_quarry_rewards_share(100000));
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
            total_rewards_per_day += u64::try_from(
                rewarder.compute_quarry_annual_rewards_rate(quarry_rewards_shares[i as usize]),
            )
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
            assert_eq!(rewarder.compute_quarry_annual_rewards_rate(quarry_rewards_share), 0);
        }
    }

    proptest! {
        #[test]
        fn test_compute_quarry_rewards_rate_with_multiple_quarries(
            annual_rewards_rate in SECONDS_PER_DAY as u64..MAX_REWARDS_RATE,
            num_quarries in 1..MAX_QUARRIES,
        ) {
            let rewarder = &mut Rewarder::default();
            rewarder.annual_rewards_rate = annual_rewards_rate * num_quarries;

            let mut rng = thread_rng();
            let mut quarry_rewards_shares: Vec<u64> = Vec::new();
            for _ in 0..num_quarries {
                let quarry_rewards_share: u32 = rng.gen_range(1..annual_rewards_rate as u32 / 10);
                add_quarry(rewarder, quarry_rewards_share as u64);
                quarry_rewards_shares.push(quarry_rewards_share.into());
            }

            let min_quarry_rewards_share = *quarry_rewards_shares.iter().min().unwrap();
            prop_assume!(rewarder.annual_rewards_rate.checked_mul(min_quarry_rewards_share).is_some());

            let mut total_rewards_per_day: u128 = 0;
            for i in 0..num_quarries {
                total_rewards_per_day += rewarder.compute_quarry_annual_rewards_rate(quarry_rewards_shares[i as usize]);
            }
            let diff = rewarder.annual_rewards_rate - u64::try_from(total_rewards_per_day).unwrap();

            const MAX_EPSILON: u64 = 200;
            let epsilon = if diff > num_quarries / 2 {
                diff - num_quarries / 2
            } else {
                num_quarries / 2 - diff
            };
            assert!(epsilon <= MAX_EPSILON, "diff: {}, num_quarries / 2: {}, epsilon: {}", diff, num_quarries / 2, epsilon);
        }
    }
}
