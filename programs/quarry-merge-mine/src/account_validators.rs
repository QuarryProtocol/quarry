//! Account validators

use anchor_lang::prelude::*;
use vipers::validate::Validate;
use vipers::{assert_ata, assert_keys, invariant};

use crate::ClaimRewards;
use crate::WithdrawTokens;
use crate::{InitMergeMiner, QuarryStakePrimary};
use crate::{InitMiner, QuarryStake};
use crate::{NewPool, QuarryStakeReplica};
use anchor_lang::Key;

/// --------------------------------
/// Instruction account structs
/// --------------------------------

impl<'info> Validate<'info> for NewPool<'info> {
    fn validate(&self) -> ProgramResult {
        // replica_mint checks are now redundant
        // since it is now an associated mint
        assert_keys!(
            self.replica_mint.mint_authority.unwrap(),
            self.pool,
            "replica_mint.mint_authority"
        );
        invariant!(
            self.replica_mint.freeze_authority.is_none(),
            "cannot have freeze authority"
        );
        invariant!(
            self.replica_mint.decimals == self.primary_mint.decimals,
            "decimals mismatch"
        );
        invariant!(
            self.replica_mint.supply == 0,
            "replica must have zero supply"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for InitMergeMiner<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for InitMiner<'info> {
    fn validate(&self) -> ProgramResult {
        require!(
            self.quarry.token_mint_key == self.pool.primary_mint
                || self.quarry.token_mint_key == self.pool.replica_mint,
            InvalidMiner
        );

        assert_keys!(self.mm.pool, self.pool, "mm.pool");
        assert_keys!(
            self.quarry.rewarder_key,
            *self.rewarder,
            "quarry.rewarder_key"
        );
        assert_ata!(
            self.miner_vault,
            self.miner,
            self.quarry.token_mint_key,
            "miner_vault"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for WithdrawTokens<'info> {
    fn validate(&self) -> ProgramResult {
        let withdraw_mint = self.mm_token_account.mint;

        assert_keys!(self.withdraw_mint, withdraw_mint, "withdraw_mint");
        // cannot withdraw a replica mint.
        require!(
            self.withdraw_mint.key() != self.pool.replica_mint,
            CannotWithdrawReplicaMint
        );

        if withdraw_mint == self.pool.primary_mint {
            // should be no replica balance if withdrawing primary
            require!(self.mm.replica_balance == 0, OutstandingReplicaTokens);
        }

        assert_keys!(self.owner, self.mm.owner, "owner");
        assert_keys!(self.pool, self.mm.pool, "pool");

        assert_ata!(
            self.mm_token_account,
            self.mm,
            withdraw_mint,
            "mm_token_account"
        );
        assert_keys!(
            self.token_destination.mint,
            withdraw_mint,
            "token_destination.mint"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for ClaimRewards<'info> {
    fn validate(&self) -> ProgramResult {
        self.stake.validate()?;

        assert_keys!(
            self.minter.mint_wrapper,
            *self.mint_wrapper,
            "minter.mint_wrapper"
        );
        assert_keys!(
            self.minter.minter_authority,
            *self.stake.rewarder,
            "minter.minter_authority"
        );
        assert_keys!(
            *self.rewards_token_mint,
            self.mint_wrapper.token_mint,
            "mint_wrapper.token_mint"
        );
        assert_ata!(
            *self.rewards_token_account,
            self.stake.mm,
            *self.rewards_token_mint,
            "rewards_token_account"
        );
        assert_keys!(
            self.claim_fee_token_account.mint,
            *self.rewards_token_mint,
            "claim_fee_token_account.mint"
        );
        assert_keys!(
            self.stake_token_account.mint,
            self.stake.quarry.token_mint_key,
            "stake_token_account.mint"
        );

        Ok(())
    }
}

/// --------------------------------
/// Context Structs
/// --------------------------------

impl<'info> Validate<'info> for QuarryStakePrimary<'info> {
    fn validate(&self) -> ProgramResult {
        self.stake.validate()?;

        // For primary staking:
        // - quarry is a quarry, staking the `primary_mint`

        assert_keys!(self.mm_owner, self.stake.mm.owner, "mm_owner");
        assert_keys!(
            self.stake.quarry.token_mint_key,
            self.stake.pool.primary_mint,
            "stake.quarry.token_mint_key"
        );
        assert_ata!(
            self.mm_primary_token_account,
            self.stake.mm,
            self.stake.pool.primary_mint,
            "mm_primary_token_account"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for QuarryStakeReplica<'info> {
    fn validate(&self) -> ProgramResult {
        self.stake.validate()?;

        // For replica staking:
        // - rewarder is any rewarder
        // - quarry is a quarry staking the `replica_mint`
        assert_keys!(self.mm_owner, self.stake.mm.owner, "mm_owner");

        assert_keys!(
            self.stake.pool.replica_mint,
            self.replica_mint,
            "stake.pool.replica_mint"
        );
        assert_keys!(
            self.stake.quarry.token_mint_key,
            self.replica_mint,
            "stake.quarry.token_mint_key"
        );
        assert_ata!(
            self.replica_mint_token_account,
            self.stake.mm,
            self.replica_mint,
            "replica_mint_token_account"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for QuarryStake<'info> {
    fn validate(&self) -> ProgramResult {
        assert_keys!(self.mm.pool, self.pool, "mm.pool");

        assert_keys!(*self.rewarder, self.quarry.rewarder_key, "rewarder");
        assert_keys!(*self.quarry, self.miner.quarry_key, "quarry");
        assert_keys!(self.miner.authority, self.mm, "miner.authority");
        assert_keys!(self.miner_vault, self.miner.token_vault_key, "miner_vault");
        assert_ata!(
            self.miner_vault,
            *self.miner,
            self.quarry.token_mint_key,
            "miner_vault"
        );

        Ok(())
    }
}
