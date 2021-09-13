//! Calculates token distribution rates.

use crate::{Miner, Quarry};
use anchor_lang::{
    prelude::{msg, ProgramError, ProgramResult},
    require,
};
use spl_math::uint::U192;
use std::cmp;
use std::convert::TryInto;
use vipers::unwrap_int;

/// Number of seconds in a year.
pub const SECONDS_PER_YEAR: u128 = 86_400 * 365;

/// Number of decimal points of precision that `rewards_per_token_stored` uses.
pub const PRECISION_MULTIPLIER: u128 = u64::MAX as u128;

/// Calculator for amount of tokens to pay out.
#[derive(Debug)]
pub struct Payroll {
    /// Timestamp of when rewards should end.
    pub famine_ts: i64,
    /// Timestamp of the last update.
    pub last_checkpoint_ts: i64,

    /// Amount of tokens to issue per year.
    pub annual_rewards_rate: u64,

    /// Amount of tokens to issue per staked token,
    /// multiplied by u64::MAX for precision.
    pub rewards_per_token_stored: u128,

    /// Total number of tokens deposited into the [Quarry].
    pub total_tokens_deposited: u64,
}

impl From<Quarry> for Payroll {
    /// Create a [Payroll] from a [Quarry].
    fn from(quarry: Quarry) -> Self {
        Self::new(
            quarry.famine_ts,
            quarry.last_update_ts,
            quarry.annual_rewards_rate,
            quarry.rewards_per_token_stored,
            quarry.total_tokens_deposited,
        )
    }
}

impl Payroll {
    /// Creates a new [Payroll].
    pub fn new(
        famine_ts: i64,
        last_checkpoint_ts: i64,
        annual_rewards_rate: u64,
        rewards_per_token_stored: u128,
        total_tokens_deposited: u64,
    ) -> Self {
        Self {
            famine_ts,
            last_checkpoint_ts,
            annual_rewards_rate,
            rewards_per_token_stored,
            total_tokens_deposited,
        }
    }

    /// Calculates the amount of rewards to pay out for each staked token.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L62
    fn calculate_reward_per_token_unsafe(&self, current_ts: i64) -> Option<u128> {
        if self.total_tokens_deposited == 0 {
            Some(self.rewards_per_token_stored)
        } else {
            let time_worked = self.compute_time_worked(current_ts)?;

            let reward = U192::from(time_worked)
                .checked_mul(PRECISION_MULTIPLIER.into())?
                .checked_mul(self.annual_rewards_rate.into())?
                .checked_div(SECONDS_PER_YEAR.into())?
                .checked_div(self.total_tokens_deposited.into())?;

            let precise_reward: u128 = reward.try_into().ok()?;

            self.rewards_per_token_stored.checked_add(precise_reward)
        }
    }

    /// Calculates the amount of rewards to pay for each staked token, performing safety checks.
    pub fn calculate_reward_per_token(&self, current_ts: i64) -> Result<u128, ProgramError> {
        require!(current_ts >= self.last_checkpoint_ts, InvalidTimestamp);
        Ok(unwrap_int!(
            self.calculate_reward_per_token_unsafe(current_ts)
        ))
    }

    /// Calculates the amount of rewards earned for the given number of staked tokens.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L72
    fn calculate_rewards_earned_unsafe(
        &self,
        current_ts: i64,
        tokens_deposited: u64,
        rewards_per_token_paid: u128,
        rewards_earned: u64,
    ) -> Option<u128> {
        let net_new_rewards = self
            .calculate_reward_per_token_unsafe(current_ts)?
            .checked_sub(rewards_per_token_paid)?;
        let rewards_earned = U192::from(tokens_deposited)
            .checked_mul(net_new_rewards.into())?
            .checked_div(PRECISION_MULTIPLIER.into())?
            .checked_add(rewards_earned.into())?;

        let precise_rewards_earned: u128 = rewards_earned.try_into().ok()?;
        Some(precise_rewards_earned)
    }

    /// Calculates the amount of rewards earned for the given number of staked tokens, with safety checks.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L72
    pub fn calculate_rewards_earned(
        &self,
        current_ts: i64,
        tokens_deposited: u64,
        rewards_per_token_paid: u128,
        rewards_earned: u64,
    ) -> Result<u128, ProgramError> {
        require!(
            tokens_deposited <= self.total_tokens_deposited,
            NotEnoughTokens
        );
        require!(current_ts >= self.last_checkpoint_ts, InvalidTimestamp);
        let result = unwrap_int!(self.calculate_rewards_earned_unsafe(
            current_ts,
            tokens_deposited,
            rewards_per_token_paid,
            rewards_earned,
        ),);
        Ok(result)
    }

    fn calculate_claimable_upper_bound_unsafe(
        &self,
        current_ts: i64,
        rewards_per_token_paid: u128,
    ) -> Option<U192> {
        let time_worked = self.compute_time_worked(current_ts)?;

        let quarry_rewards_accrued = U192::from(time_worked)
            .checked_mul(self.annual_rewards_rate.into())?
            .checked_div(SECONDS_PER_YEAR.into())?;

        let net_rewards_per_token = self
            .rewards_per_token_stored
            .checked_sub(rewards_per_token_paid)?;
        let net_quarry_rewards = U192::from(net_rewards_per_token)
            .checked_mul(self.total_tokens_deposited.into())?
            .checked_div(PRECISION_MULTIPLIER.into())?;

        quarry_rewards_accrued.checked_add(net_quarry_rewards)
    }

    /// Sanity check on the amount of rewards to be claimed by the miner.
    pub fn sanity_check(
        &self,
        current_ts: i64,
        amount_claimable: u64,
        miner: &Miner,
    ) -> ProgramResult {
        let rewards_upperbound =
            unwrap_int!(self
                .calculate_claimable_upper_bound_unsafe(current_ts, miner.rewards_per_token_paid,));
        let amount_claimable_less_already_earned =
            unwrap_int!(amount_claimable.checked_sub(miner.rewards_earned));

        if rewards_upperbound < amount_claimable_less_already_earned.into() {
            msg!(
                "current_ts: {}, rewards_upperbound: {}, amount_claimable: {}, payroll: {:?}, miner: {:?}",
                current_ts,
                rewards_upperbound,
                amount_claimable,
                self,
                miner,
            );
            require!(
                rewards_upperbound + 1 >= amount_claimable.into(), // Allow off by one.
                UpperboundExceeded
            );
        }

        Ok(())
    }

    /// Gets the latest time rewards were being distributed.
    pub fn last_time_reward_applicable(&self, current_ts: i64) -> i64 {
        cmp::min(current_ts, self.famine_ts)
    }

    /// Calculates the amount of seconds the [Payroll] should have applied rewards for.
    fn compute_time_worked(&self, current_ts: i64) -> Option<i64> {
        Some(cmp::max(
            0,
            self.last_time_reward_applicable(current_ts)
                .checked_sub(self.last_checkpoint_ts)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::MAX_ANNUAL_REWARDS_RATE;
    /// Maximum seconds elapsed between two checkpoints.
    /// [i32::MAX] corresponds to about 70 years.
    const MAX_SECONDS_BETWEEN_CHECKPOINTS: i64 = i32::MAX as i64;
    const MAX_TOTAL_TOKENS: u64 = 1_000_000_000_000_000;

    use super::*;
    use num_traits::ToPrimitive;
    use proptest::prelude::*;

    macro_rules! assert_percent_delta {
        ($x:expr, $y:expr, $d:expr) => {
            let delta = if $x > $y {
                $x - $y
            } else if $y > $x {
                $y - $x
            } else {
                0
            };
            let delta_f = if delta == 0 && $y == 0 {
                0.0_f64
            } else {
                (delta as f64) / ($y as f64)
            };
            assert!(
                delta_f < $d,
                "Delta {} > {}; left: {}, right: {}",
                delta_f,
                $d,
                $x,
                $y
            );
        };
    }

    prop_compose! {
        pub fn part_and_total_small()(
            total in 0..u64::MAX
        )(
            // use a really small number here
            part in 0..cmp::min(100_000_u64, total),
            total in Just(total)
        ) -> (u64, u64) {
           (part, total)
       }
    }

    prop_compose! {
        pub fn part_and_total()(
            total in 0..MAX_TOTAL_TOKENS
        )(
            // use a really small number here
            part in 0..total,
            total in Just(total)
        ) -> (u64, u64) {
           (part, total)
       }
    }

    proptest! {
        /// Precision errors should not be over EPSILON.
        #[test]
        fn test_accumulated_precision_errors_epsilon(
            num_updates in 1..100_i64,
            (final_ts, initial_ts) in total_and_intermediate_ts(),
            annual_rewards_rate in 0..=MAX_ANNUAL_REWARDS_RATE,
            (my_tokens_deposited, total_tokens_deposited) in part_and_total_small()
        ) {
            const EPSILON: f64 = 0.0001;

            let mut rewards_per_token_stored: u128 = 0;
            let mut last_checkpoint_ts = initial_ts;
            for i in 0..=num_updates {
                let payroll = Payroll::new(
                    i64::MAX,
                    last_checkpoint_ts,
                    annual_rewards_rate,
                    rewards_per_token_stored,
                    total_tokens_deposited
                );
                let current_ts = initial_ts + (((final_ts - initial_ts) as u128) * (i as u128) / (num_updates as u128)).to_i64().unwrap();
                rewards_per_token_stored = payroll.calculate_reward_per_token(current_ts).unwrap();
                last_checkpoint_ts = current_ts;
            }

            let payroll = Payroll::new(
                i64::MAX,
                last_checkpoint_ts,
                annual_rewards_rate,
                rewards_per_token_stored,
                total_tokens_deposited
            );
            let rewards_earned = payroll.calculate_rewards_earned(
                final_ts,
                my_tokens_deposited,
                0_u128,
                0
            ).unwrap();

            let expected_rewards_earned = U192::from(annual_rewards_rate)
                * U192::from(final_ts - initial_ts)
                * U192::from(my_tokens_deposited)
                / U192::from(SECONDS_PER_YEAR)
                / U192::from(total_tokens_deposited);

            assert_percent_delta!(expected_rewards_earned.as_u128(), rewards_earned, EPSILON);
        }
    }

    proptest! {
        #[test]
        fn test_sanity_check(
            annual_rewards_rate in 0..=MAX_ANNUAL_REWARDS_RATE,
            rewards_already_earned in u64::MIN..MAX_TOTAL_TOKENS,
            (rewards_per_token_paid, rewards_per_token_stored) in part_and_total(),
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            (my_tokens_deposited, total_tokens_deposited) in part_and_total()
        ) {
            let payroll = Payroll::new(
                i64::MAX,
                last_checkpoint_ts,
                annual_rewards_rate,
                rewards_per_token_stored as u128,
                total_tokens_deposited
            );

            let amount_claimable_less_already_earned = payroll.calculate_rewards_earned(current_ts, my_tokens_deposited, rewards_per_token_paid.into(), rewards_already_earned).unwrap() - rewards_already_earned as u128;
            let upperbound = payroll.calculate_claimable_upper_bound_unsafe(current_ts, rewards_per_token_paid.into()).unwrap();

            assert!(upperbound >= amount_claimable_less_already_earned.into(), "amount_claimable_less_already_earned: {}, upperbound: {}", amount_claimable_less_already_earned, upperbound);
        }
    }

    #[test]
    fn test_sanity_check_off_by_one_case() {
        // FIXME: Find out why sometimes upperbound can be off by one.
        let total_tokens_deposited = 1_000_000;
        let annual_rewards_rate = 365_000_000_000_000;
        let rewards_per_token_stored: u128 = 576247267536447296791024;

        let last_checkpoint_ts = 0;
        let payroll = Payroll::new(
            i64::MAX,
            last_checkpoint_ts,
            annual_rewards_rate,
            rewards_per_token_stored,
            total_tokens_deposited,
        );

        let current_ts = 6;
        let rewards_earned = payroll
            .calculate_rewards_earned(current_ts, total_tokens_deposited, 0, 0)
            .unwrap();
        let upperbound = payroll
            .calculate_claimable_upper_bound_unsafe(current_ts, 0)
            .unwrap();

        assert_eq!(
            upperbound + 1,
            rewards_earned.into(),
            "rewards_earned: {}, upperbound: {}",
            rewards_earned,
            upperbound
        );
    }

    proptest! {
        #[test]
        fn test_wpt_with_zero_annual_rewards_rate(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            rewards_per_token_stored in u64::MIN..u64::MAX,
            total_tokens_deposited in u64::MIN..u64::MAX,
        ) {
            let payroll = Payroll::new(famine_ts, last_checkpoint_ts, 0, rewards_per_token_stored.into(), total_tokens_deposited);
            assert_eq!(payroll.calculate_reward_per_token(current_ts).unwrap(), rewards_per_token_stored.into())
        }
    }

    proptest! {
        #[test]
        fn test_wpt_when_famine(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            annual_rewards_rate in 1..u64::MAX,
            rewards_per_token_stored in u64::MIN..u64::MAX,
            total_tokens_deposited in u64::MIN..u64::MAX,
        ) {
            let payroll = Payroll::new(
                famine_ts, last_checkpoint_ts, annual_rewards_rate,
                rewards_per_token_stored.into(), total_tokens_deposited
            );
            prop_assume!(famine_ts < current_ts && famine_ts < last_checkpoint_ts);
            assert_eq!(payroll.calculate_reward_per_token(current_ts).unwrap(), rewards_per_token_stored.into())
        }
    }

    proptest! {
        #[test]
        fn test_rewards_earned_when_zero_tokens_deposited(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            annual_rewards_rate in 0..u64::MAX,
            rewards_per_token_stored in u64::MIN..u64::MAX,
            total_tokens_deposited in u64::MIN..u64::MAX,
            rewards_per_token_paid in u64::MIN..u64::MAX,
            rewards_earned in u64::MIN..u64::MAX,
        ) {
            let payroll = Payroll::new(famine_ts, last_checkpoint_ts, annual_rewards_rate, rewards_per_token_stored.into(), total_tokens_deposited);
            prop_assume!(payroll.calculate_reward_per_token(current_ts).unwrap() >= rewards_per_token_paid.into());
            assert_eq!(payroll.calculate_rewards_earned(current_ts, 0, rewards_per_token_paid.into(), rewards_earned).unwrap(), rewards_earned.into())
        }
    }

    prop_compose! {
        pub fn total_and_intermediate_ts()(
          elapsed_seconds in 0..MAX_SECONDS_BETWEEN_CHECKPOINTS,
          last_checkpoint_ts in 0..(i64::MAX - MAX_SECONDS_BETWEEN_CHECKPOINTS),
        ) -> (i64, i64) {
          (last_checkpoint_ts + elapsed_seconds, last_checkpoint_ts)
       }
    }
}
