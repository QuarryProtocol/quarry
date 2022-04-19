//! Account validators

use anchor_lang::prelude::*;
use vipers::prelude::*;

use crate::{
    ClaimRewards, InitMergeMiner, InitMiner, NewPool, QuarryStake, QuarryStakePrimary,
    QuarryStakeReplica, WithdrawTokens,
};
use anchor_lang::Key;

// --------------------------------
// Instruction account validators
// --------------------------------

impl<'info> Validate<'info> for NewPool<'info> {
    fn validate(&self) -> Result<()> {
        // replica_mint checks are now redundant
        // since it is now an associated mint
        assert_keys_eq!(self.replica_mint.mint_authority.unwrap(), self.pool);
        invariant!(self.replica_mint.freeze_authority.is_none());
        invariant!(
            self.replica_mint.decimals == self.primary_mint.decimals,
            ReplicaDecimalsMismatch
        );
        invariant!(self.replica_mint.supply == 0, ReplicaNonZeroSupply);

        Ok(())
    }
}

impl<'info> Validate<'info> for InitMergeMiner<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl<'info> Validate<'info> for InitMiner<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(
            self.quarry.token_mint_key == self.pool.primary_mint
                || self.quarry.token_mint_key == self.pool.replica_mint,
            InvalidMiner
        );

        assert_keys_eq!(self.mm.pool, self.pool);
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        assert_keys_eq!(self.miner_vault.owner, self.miner);
        assert_keys_eq!(self.miner_vault.mint, self.quarry.token_mint_key);
        invariant!(self.miner_vault.delegate.is_none());
        invariant!(self.miner_vault.close_authority.is_none());

        Ok(())
    }
}

impl<'info> Validate<'info> for WithdrawTokens<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.owner, self.mm.owner, Unauthorized);
        assert_keys_eq!(self.pool, self.mm.pool);

        let withdraw_mint = self.mm_token_account.mint;

        assert_keys_eq!(self.withdraw_mint, withdraw_mint);
        // cannot withdraw a replica mint.
        assert_keys_neq!(
            self.withdraw_mint,
            self.pool.replica_mint,
            CannotWithdrawReplicaMint
        );

        assert_keys_neq!(self.mm_token_account, self.token_destination);

        if withdraw_mint == self.pool.primary_mint {
            // should be no replica balance if withdrawing primary
            invariant!(self.mm.replica_balance == 0, OutstandingReplicaTokens);
        }

        assert_keys_eq!(self.mm_token_account.mint, withdraw_mint);
        assert_keys_eq!(self.mm_token_account.owner, self.mm);
        invariant!(self.mm_token_account.delegate.is_none());
        invariant!(self.mm_token_account.close_authority.is_none());

        assert_keys_eq!(self.token_destination.mint, withdraw_mint);

        Ok(())
    }
}

impl<'info> Validate<'info> for ClaimRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.stake.validate()?;

        assert_keys_eq!(self.mint_wrapper, self.stake.rewarder.mint_wrapper);
        assert_keys_eq!(self.minter.mint_wrapper, self.mint_wrapper);
        assert_keys_eq!(self.minter.minter_authority, self.stake.rewarder);
        assert_keys_eq!(self.rewards_token_mint, self.mint_wrapper.token_mint);
        assert_keys_eq!(
            self.rewards_token_mint,
            self.stake.rewarder.rewards_token_mint
        );

        assert_keys_eq!(self.rewards_token_account.mint, self.rewards_token_mint);
        assert_keys_eq!(self.rewards_token_account.owner, self.stake.mm);

        assert_keys_eq!(
            self.claim_fee_token_account,
            self.stake.rewarder.claim_fee_token_account
        );
        assert_keys_eq!(self.claim_fee_token_account.mint, self.rewards_token_mint);

        Ok(())
    }
}

/// --------------------------------
/// Context Structs
/// --------------------------------

impl<'info> Validate<'info> for QuarryStakePrimary<'info> {
    fn validate(&self) -> Result<()> {
        self.stake.validate()?;

        assert_keys_eq!(self.mm_owner, self.stake.mm.owner);

        // For primary staking:
        // - quarry is a quarry, staking the `primary_mint`

        let primary_mint = self.stake.quarry.token_mint_key;
        assert_keys_eq!(primary_mint, self.stake.pool.primary_mint);
        assert_keys_eq!(primary_mint, self.mm_primary_token_account.mint);

        assert_keys_eq!(self.mm_primary_token_account.owner, self.stake.mm);
        invariant!(self.mm_primary_token_account.delegate.is_none());
        invariant!(self.mm_primary_token_account.close_authority.is_none());

        Ok(())
    }
}

impl<'info> Validate<'info> for QuarryStakeReplica<'info> {
    fn validate(&self) -> Result<()> {
        self.stake.validate()?;

        // For replica staking:
        // - rewarder is any rewarder
        // - quarry is a quarry staking the `replica_mint`
        let replica_mint_key = self.replica_mint.key();
        assert_keys_eq!(self.mm_owner, self.stake.mm.owner);

        assert_keys_eq!(self.stake.pool.replica_mint, replica_mint_key);
        assert_keys_eq!(self.stake.quarry.token_mint_key, replica_mint_key);

        assert_keys_eq!(self.replica_mint_token_account.mint, self.replica_mint);
        assert_keys_eq!(self.replica_mint_token_account.owner, self.stake.mm);
        invariant!(self.replica_mint_token_account.delegate.is_none());
        invariant!(self.replica_mint_token_account.close_authority.is_none());

        Ok(())
    }
}

impl<'info> Validate<'info> for QuarryStake<'info> {
    fn validate(&self) -> Result<()> {
        // this links merge miner validations with quarry validations.
        let quarry_mint = self.quarry.token_mint_key;
        invariant!(self.pool.primary_mint == quarry_mint || self.pool.replica_mint == quarry_mint);

        // merge miner validations
        assert_keys_eq!(self.mm, self.miner.authority);
        assert_keys_eq!(self.pool, self.mm.pool);

        // Quarry validations
        assert_keys_eq!(self.quarry, self.miner.quarry);
        assert_keys_eq!(self.rewarder, self.quarry.rewarder);

        assert_keys_eq!(self.miner_vault, self.miner.token_vault_key);
        assert_keys_eq!(self.miner_vault.owner, self.miner);
        assert_keys_eq!(self.miner_vault.mint, quarry_mint);
        invariant!(self.miner_vault.delegate.is_none());
        invariant!(self.miner_vault.close_authority.is_none());

        Ok(())
    }
}
