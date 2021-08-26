//! Quarry-related math and helpers.

use anchor_lang::prelude::*;
use vipers::unwrap_int;

use crate::{payroll::Payroll, Miner, Quarry, Rewarder};
use num_traits::cast::ToPrimitive;

pub enum StakeAction {
    Stake,
    Withdraw,
}

impl Quarry {
    /// Updates the quarry by synchronizing its rewards rate with the rewarder.
    pub fn update_rewards_internal(
        &mut self,
        current_ts: i64,
        rewarder: &Rewarder,
        payroll: &Payroll,
    ) -> ProgramResult {
        let updated_rewards_per_token_stored =
            unwrap_int!(payroll.calculate_reward_per_token(current_ts)?.to_u64());
        // Update quarry struct
        self.rewards_per_token_stored = updated_rewards_per_token_stored;
        self.daily_rewards_rate = unwrap_int!(rewarder
            .compute_quarry_daily_rewards_rate(self.rewards_share)
            .to_u64());
        self.last_update_ts = payroll.last_time_reward_applicable(current_ts);

        Ok(())
    }

    /// Updates the quarry and miner with the latest info.
    /// https://github.com/Synthetixio/synthetix/blob/aeee6b2c82588681e1f99202663346098d1866ac/contracts/StakingRewards.sol#L158
    pub fn update_rewards_and_miner(
        &mut self,
        miner: &mut Miner,
        rewarder: &Rewarder,
        current_ts: i64,
    ) -> ProgramResult {
        let payroll: Payroll = (*self).into();
        self.update_rewards_internal(current_ts, rewarder, &payroll)?;

        let updated_rewards_earned = unwrap_int!(payroll
            .calculate_rewards_earned(
                current_ts,
                miner.balance.into(),
                miner.rewards_per_token_paid.into(),
                miner.rewards_earned.into(),
            )?
            .to_u64());
        // Update miner struct
        miner.rewards_earned = updated_rewards_earned;
        miner.rewards_per_token_paid = self.rewards_per_token_stored;

        Ok(())
    }

    pub fn process_stake_action_internal(
        &mut self,
        action: StakeAction,
        current_ts: i64,
        lord: &Rewarder,
        miner: &mut Miner,
        amount: u64,
    ) -> ProgramResult {
        self.update_rewards_and_miner(miner, lord, current_ts)?;
        match action {
            StakeAction::Stake => {
                miner.balance = unwrap_int!(miner.balance.checked_add(amount));
                self.total_tokens_deposited =
                    unwrap_int!(self.total_tokens_deposited.checked_add(amount));
            }
            StakeAction::Withdraw => {
                miner.balance = unwrap_int!(miner.balance.checked_sub(amount));
                self.total_tokens_deposited =
                    unwrap_int!(self.total_tokens_deposited.checked_sub(amount));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{payroll::SECONDS_PER_DAY, quarry::StakeAction};

    const DEFAULT_TOKEN_DECIMALS: u8 = 6;

    pub struct MinerVault {
        balance: u64,
    }

    fn sim_claim(
        current_ts: i64,
        rewarder: &Rewarder,
        quarry: &mut Quarry,
        _vault: &mut MinerVault,
        miner: &mut Miner,
    ) -> u64 {
        quarry
            .update_rewards_and_miner(miner, rewarder, current_ts)
            .unwrap();
        let amount_claimable = miner.rewards_earned;
        miner.rewards_earned = 0;

        amount_claimable
    }

    fn sim_stake(
        current_ts: i64,
        rewarder: &Rewarder,
        quarry: &mut Quarry,
        vault: &mut MinerVault,
        miner: &mut Miner,
        amount: u64,
    ) {
        quarry
            .process_stake_action_internal(StakeAction::Stake, current_ts, rewarder, miner, amount)
            .unwrap();
        vault.balance += amount;
    }

    fn sim_withdraw(
        current_ts: i64,
        rewarder: &Rewarder,
        quarry: &mut Quarry,
        vault: &mut MinerVault,
        miner: &mut Miner,
        amount: u64,
    ) {
        quarry
            .process_stake_action_internal(
                StakeAction::Withdraw,
                current_ts,
                rewarder,
                miner,
                amount,
            )
            .unwrap();
        vault.balance -= amount;
    }

    fn to_unit(amount: u64) -> u64 {
        amount * u64::pow(10, DEFAULT_TOKEN_DECIMALS.into())
    }

    #[test]
    fn test_lifecycle_one_miner() {
        let quarry = &mut Quarry::default();
        quarry.famine_ts = i64::MAX;
        quarry.rewards_share = 100;
        quarry.token_mint_decimals = DEFAULT_TOKEN_DECIMALS;
        let miner_vault = &mut MinerVault { balance: 0 };

        let rewarder = Rewarder {
            bump: 254,
            daily_rewards_rate: to_unit(5000),
            num_quarries: 1,
            total_rewards_shares: quarry.rewards_share,
            ..Default::default()
        };

        let miner = &mut Miner::default();

        let mut current_ts: i64 = 0;
        let total_to_stake = to_unit(500);

        // Stake tokens
        sim_stake(
            current_ts,
            &rewarder,
            quarry,
            miner_vault,
            miner,
            total_to_stake,
        );
        assert!(quarry.daily_rewards_rate > 0);
        assert_eq!(miner_vault.balance, total_to_stake);

        // Fastforward time by 6 days
        current_ts += SECONDS_PER_DAY as i64 * 6;
        let expected_rewards_earned = 29999808000; // About 500 * 10 ^ 6 * 6

        // Withdraw half
        let withdraw_amount = to_unit(250);
        sim_withdraw(
            current_ts,
            &rewarder,
            quarry,
            miner_vault,
            miner,
            withdraw_amount,
        );
        assert!(quarry.rewards_per_token_stored > 0);
        assert_eq!(
            miner.rewards_earned,
            miner.rewards_per_token_paid * total_to_stake / to_unit(1)
        );
        assert_eq!(miner.rewards_earned, expected_rewards_earned);
        assert_eq!(miner_vault.balance, total_to_stake - withdraw_amount);

        // Claim rewards
        let expected_rewards_earned = miner.rewards_earned;
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault, miner),
            expected_rewards_earned
        );
        // Should not allow double claim
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault, miner),
            0
        );

        // Fastforward time another 6 days
        current_ts += SECONDS_PER_DAY as i64 * 6;

        // Withdraw remaining half
        sim_withdraw(
            current_ts,
            &rewarder,
            quarry,
            miner_vault,
            miner,
            withdraw_amount,
        );
        assert_eq!(miner_vault.balance, 0);

        // Claim rewards, still the same since we're the only miner in the quarry
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault, miner),
            expected_rewards_earned
        );

        // Fastforward time by 6 days
        current_ts += SECONDS_PER_DAY as i64 * 6;

        // Claim rewards again, should be 0 since all tokens were withdrawn
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault, miner),
            0
        );
    }

    #[test]
    fn test_lifecycle_two_miners() {
        let quarry = &mut Quarry::default();
        quarry.famine_ts = i64::MAX;
        quarry.rewards_share = 100;
        quarry.token_mint_decimals = DEFAULT_TOKEN_DECIMALS;
        let miner_vault_one = &mut MinerVault { balance: 0 };
        let miner_vault_two = &mut MinerVault { balance: 0 };
        let rewarder = Rewarder {
            bump: 254,
            daily_rewards_rate: to_unit(5000),
            num_quarries: 1,
            total_rewards_shares: quarry.rewards_share,
            ..Default::default()
        };
        let miner_one = &mut Miner::default();
        let miner_two = &mut Miner::default();

        let mut current_ts: i64 = 0;
        let total_to_stake = to_unit(500);

        // Stake tokens
        sim_stake(
            current_ts,
            &rewarder,
            quarry,
            miner_vault_one,
            miner_one,
            total_to_stake,
        );
        assert_eq!(miner_vault_one.balance, total_to_stake);
        assert_eq!(miner_one.balance, miner_vault_one.balance);
        sim_stake(
            current_ts,
            &rewarder,
            quarry,
            miner_vault_two,
            miner_two,
            total_to_stake,
        );
        assert_eq!(miner_vault_two.balance, total_to_stake);
        assert_eq!(miner_two.balance, miner_vault_two.balance);
        assert!(quarry.daily_rewards_rate > 0);

        // Fastforward time by 3 days
        current_ts += SECONDS_PER_DAY as i64 * 3;

        // Miner two withdraws their stake
        sim_withdraw(
            current_ts,
            &rewarder,
            quarry,
            miner_vault_two,
            miner_two,
            total_to_stake,
        );
        assert!(quarry.rewards_per_token_stored > 0);
        assert_eq!(
            miner_two.rewards_earned,
            miner_two.rewards_per_token_paid * total_to_stake / to_unit(1)
        );
        assert_eq!(miner_vault_two.balance, 0);
        assert_eq!(miner_two.balance, miner_vault_two.balance);

        // Fastforward time by 3 days
        current_ts += SECONDS_PER_DAY as i64 * 3;

        // Claim rewards
        let expected_miner_one_rewards_earned = 22499856000; // About 3000 * 10 ^ 6 * (3/4)
        let expected_miner_two_rewards_earned = 7499952000; // About 3000 * 10 ^ 6 * (1/4)
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_one, miner_one),
            expected_miner_one_rewards_earned
        );
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_two, miner_two),
            expected_miner_two_rewards_earned
        );

        // Fastforward time by 6 days
        current_ts += SECONDS_PER_DAY as i64 * 6;

        // Claim rewards
        let expected_miner_one_rewards_earned = 29999808000;
        let expected_miner_two_rewards_earned = 0;
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_one, miner_one),
            expected_miner_one_rewards_earned
        );
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_two, miner_two),
            expected_miner_two_rewards_earned
        );

        // Miner two re-stakes
        sim_stake(
            current_ts,
            &rewarder,
            quarry,
            miner_vault_two,
            miner_two,
            total_to_stake,
        );
        assert_eq!(miner_vault_two.balance, total_to_stake);
        assert_eq!(miner_two.balance, miner_vault_two.balance);

        // Fastforward time by 6 days
        current_ts += SECONDS_PER_DAY as i64 * 6;

        // Claim rewards
        let expected_miner_one_rewards_earned = expected_miner_one_rewards_earned / 2;
        let expected_miner_two_rewards_earned = expected_miner_one_rewards_earned;
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_one, miner_one),
            expected_miner_one_rewards_earned
        );
        assert_eq!(
            sim_claim(current_ts, &rewarder, quarry, miner_vault_two, miner_two),
            expected_miner_two_rewards_earned
        );
    }
}
