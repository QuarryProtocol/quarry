use crate::Quarry;
use anchor_lang::{prelude::ProgramError, require};
use std::cmp;
use vipers::unwrap_int;

pub const SECONDS_PER_DAY: u128 = 86_400;

/// Calculator for amount of tokens to pay out.
pub struct Payroll {
    /// Timestamp of when rewards should end.
    pub famine_ts: i64,
    /// Timestamp of the last update.
    pub last_checkpoint_ts: i64,

    /// Amount of tokens to issue per second.
    pub rewards_rate_per_second: u128,
    /// Amount of tokens to issue per staked token.
    pub rewards_per_token_stored: u128,

    /// Number of decimals on the token.
    pub token_decimals: u8,
    /// Total number of tokens deposited into the [Quarry].
    pub total_tokens_deposited: u128,
}

impl From<Quarry> for Payroll {
    /// Create a [Payroll] from a [Quarry].
    fn from(quarry: Quarry) -> Self {
        Self::new(
            quarry.famine_ts,
            quarry.last_update_ts,
            quarry.daily_rewards_rate as u128 / SECONDS_PER_DAY,
            quarry.rewards_per_token_stored.into(),
            quarry.token_mint_decimals,
            quarry.total_tokens_deposited.into(),
        )
    }
}

impl Payroll {
    /// Creates a new [Payroll].
    pub fn new(
        famine_ts: i64,
        last_checkpoint_ts: i64,
        rewards_rate_per_second: u128,
        rewards_per_token_stored: u128,
        token_decimals: u8,
        total_tokens_deposited: u128,
    ) -> Self {
        Self {
            famine_ts,
            last_checkpoint_ts,
            rewards_rate_per_second,
            rewards_per_token_stored,
            token_decimals,
            total_tokens_deposited,
        }
    }

    /// Calculates the amount of rewards to pay out for each staked token.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L62
    fn calculate_reward_per_token_unsafe(&self, current_ts: i64) -> Option<u128> {
        if self.total_tokens_deposited == 0 {
            Some(self.rewards_per_token_stored)
        } else {
            let time_worked = cmp::max(
                0,
                self.last_time_reward_applicable(current_ts)
                    .checked_sub(self.last_checkpoint_ts)?,
            );
            let reward = (time_worked as u128).checked_mul(self.rewards_rate_per_second)?;
            let precise_reward = reward
                .checked_mul(self.decimal_precision())?
                .checked_div(self.total_tokens_deposited)?;
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
        tokens_deposited: u128,
        rewards_per_token_paid: u128,
        rewards_earned: u128,
    ) -> Option<u128> {
        let net_new_rewards = self
            .calculate_reward_per_token_unsafe(current_ts)?
            .checked_sub(rewards_per_token_paid)?;
        tokens_deposited
            .checked_mul(net_new_rewards)?
            .checked_div(self.decimal_precision())?
            .checked_add(rewards_earned)
    }

    /// Calculates the amount of rewards earned for the given number of staked tokens, with safety checks.
    /// https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol#L72
    pub fn calculate_rewards_earned(
        &self,
        current_ts: i64,
        tokens_deposited: u128,
        rewards_per_token_paid: u128,
        rewards_earned: u128,
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

    pub fn last_time_reward_applicable(&self, current_ts: i64) -> i64 {
        cmp::min(current_ts, self.famine_ts)
    }

    fn decimal_precision(&self) -> u128 {
        let base = 10;
        u128::pow(base, self.token_decimals.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MAX_TOKEN_DECIMALS: u8 = 9;

    proptest! {
        #[test]
        fn test_wpt_with_zero_rewards_rate_per_second(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            rewards_per_token_stored in u64::MIN..u64::MAX,
            token_decimals in u8::MIN..MAX_TOKEN_DECIMALS,
            total_tokens_deposited in u64::MIN..u64::MAX,
        ) {
            let payroll = Payroll::new(famine_ts, last_checkpoint_ts, 0, rewards_per_token_stored.into(), token_decimals, total_tokens_deposited.into());
            assert_eq!(payroll.calculate_reward_per_token(current_ts).unwrap(), rewards_per_token_stored.into())
        }
    }

    proptest! {
        #[test]
        fn test_wpt_when_famine(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            rewards_rate_per_second in 1..u32::MAX as u128,
            rewards_per_token_stored in u64::MIN..u64::MAX,
            token_decimals in u8::MIN..MAX_TOKEN_DECIMALS,
        ) {
            const BASE: u64 = 10;
            let total_tokens_deposited = 1000000 * u64::pow(BASE, token_decimals.into());
            let payroll = Payroll::new(famine_ts, last_checkpoint_ts, rewards_rate_per_second, rewards_per_token_stored.into(), token_decimals, total_tokens_deposited.into());
            prop_assume!(famine_ts < current_ts && famine_ts < last_checkpoint_ts);
            assert_eq!(payroll.calculate_reward_per_token(current_ts).unwrap(), rewards_per_token_stored.into())
        }
    }

    proptest! {
        #[test]
        fn test_rewards_earned_when_zero_tokens_deposited(
            famine_ts in 0..i64::MAX,
            (current_ts, last_checkpoint_ts) in total_and_intermediate_ts(),
            rewards_rate_per_second in 0..u32::MAX as u128,
            rewards_per_token_stored in u64::MIN..u64::MAX,
            token_decimals in u8::MIN..MAX_TOKEN_DECIMALS,
            total_tokens_deposited in u64::MIN..u64::MAX,
            rewards_per_token_paid in u64::MIN..u64::MAX,
            rewards_earned in u64::MIN..u64::MAX,
        ) {
            let payroll = Payroll::new(famine_ts, last_checkpoint_ts, rewards_rate_per_second, rewards_per_token_stored.into(), token_decimals, total_tokens_deposited.into());
            prop_assume!(payroll.calculate_reward_per_token(current_ts).unwrap() >= rewards_per_token_paid.into());
            assert_eq!(payroll.calculate_rewards_earned(current_ts, 0, rewards_per_token_paid.into(), rewards_earned.into()).unwrap(), rewards_earned.into())
        }
    }

    prop_compose! {
        pub fn total_and_intermediate_ts()(total in 0..i64::MAX)
                        (intermediate in 0..total, total in Just(total))
                        -> (i64, i64) {
           (total, intermediate)
       }
    }
}
