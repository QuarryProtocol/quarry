//! Validations for various accounts.

use anchor_lang::prelude::*;
use vipers::prelude::*;

use crate::addresses;
use crate::{
    AcceptAuthority, ClaimRewards, CreateMiner, CreateQuarry, ExtractFees,
    MutableRewarderWithAuthority, MutableRewarderWithPauseAuthority, NewRewarder,
    ReadOnlyRewarderWithAuthority, SetAnnualRewards, SetFamine, SetPauseAuthority, SetRewardsShare,
    TransferAuthority, UpdateQuarryRewards, UserClaim, UserStake,
};

// --------------------------------
// Rewarder Functions
// --------------------------------

impl<'info> Validate<'info> for NewRewarder<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.base.is_signer, Unauthorized);

        assert_keys_eq!(self.claim_fee_token_account.owner, self.rewarder);
        assert_keys_eq!(self.claim_fee_token_account.mint, self.rewards_token_mint);
        invariant!(self.claim_fee_token_account.delegate.is_none());
        invariant!(self.claim_fee_token_account.close_authority.is_none());

        assert_keys_eq!(self.mint_wrapper.token_mint, self.rewards_token_mint);

        Ok(())
    }
}

impl<'info> Validate<'info> for SetPauseAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

impl<'info> Validate<'info> for MutableRewarderWithPauseAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.pause_authority.is_signer, Unauthorized);
        assert_keys_eq!(
            self.rewarder.pause_authority,
            self.pause_authority,
            "pause_authority"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.rewarder.authority);
        Ok(())
    }
}

impl<'info> Validate<'info> for AcceptAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        invariant!(
            self.rewarder.pending_authority != Pubkey::default(),
            PendingAuthorityNotSet
        );
        assert_keys_eq!(
            self.rewarder.pending_authority,
            self.authority,
            "pending authority"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for SetAnnualRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

// --------------------------------
// Quarry functions
// --------------------------------

impl<'info> Validate<'info> for CreateQuarry<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRewardsShare<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        assert_keys_eq!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for SetFamine<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        assert_keys_eq!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for UpdateQuarryRewards<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        assert_keys_eq!(self.quarry.rewarder_key, self.rewarder, "rewarder");
        Ok(())
    }
}

/// --------------------------------
/// Miner functions
/// --------------------------------

impl<'info> Validate<'info> for CreateMiner<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        assert_keys_eq!(self.miner_vault.owner, self.miner);
        assert_keys_eq!(self.miner_vault.mint, self.token_mint);
        invariant!(self.miner_vault.delegate.is_none());
        invariant!(self.miner_vault.close_authority.is_none());

        assert_keys_eq!(
            self.miner_vault.mint,
            self.quarry.token_mint_key,
            "miner vault mint must match quarry mint"
        );
        assert_keys_eq!(self.quarry.rewarder_key, self.rewarder, "rewarder");

        Ok(())
    }
}

impl<'info> Validate<'info> for ClaimRewards<'info> {
    /// Validates a [ClaimRewards] accounts struct.
    fn validate(&self) -> Result<()> {
        self.stake.validate()?;
        invariant!(!self.stake.rewarder.is_paused, Paused);

        assert_keys_eq!(
            self.mint_wrapper.token_mint,
            self.rewards_token_mint,
            "mint_wrapper.token_mint",
        );
        assert_keys_eq!(
            self.minter.minter_authority,
            self.stake.rewarder,
            "minter.minter_authority"
        );

        // rewards_token_mint validate
        assert_keys_eq!(
            self.rewards_token_mint,
            self.stake.rewarder.rewards_token_mint,
            "rewards token mint",
        );
        assert_keys_eq!(
            self.rewards_token_mint.mint_authority.unwrap(),
            *self.mint_wrapper,
            "mint wrapper",
        );

        // rewards_token_account validate
        assert_keys_eq!(
            self.rewards_token_account.mint,
            self.rewards_token_mint,
            "rewards_token_account.mint",
        );

        // claim_fee_token_account validate
        assert_keys_eq!(
            self.claim_fee_token_account,
            self.stake.rewarder.claim_fee_token_account
        );
        assert_keys_eq!(
            self.claim_fee_token_account.mint,
            self.rewards_token_mint,
            "rewards_token_account.mint",
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for UserClaim<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority, "miner authority");

        // quarry
        assert_keys_eq!(self.miner.quarry_key, self.quarry.key(), "quarry");

        // rewarder
        assert_keys_eq!(self.quarry.rewarder_key, self.rewarder, "rewarder");

        Ok(())
    }
}

impl<'info> Validate<'info> for UserStake<'info> {
    /// Validates the UserStake.
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority, "miner authority");

        // quarry
        assert_keys_eq!(self.miner.quarry_key, self.quarry.key(), "quarry");

        // miner_vault
        assert_keys_eq!(self.miner.token_vault_key, self.miner_vault, "miner vault");
        assert_keys_eq!(
            self.miner_vault.mint,
            self.quarry.token_mint_key,
            "vault mint"
        );
        assert_keys_eq!(self.miner_vault.owner, self.miner, "vault owner");

        // token_account
        assert_keys_eq!(
            self.token_account.mint,
            self.quarry.token_mint_key,
            "token mint"
        );

        // rewarder
        assert_keys_eq!(self.quarry.rewarder_key, self.rewarder, "rewarder");

        Ok(())
    }
}

impl<'info> Validate<'info> for ExtractFees<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);

        assert_keys_eq!(
            self.claim_fee_token_account,
            self.rewarder.claim_fee_token_account
        );
        assert_keys_eq!(
            self.claim_fee_token_account.mint,
            self.rewarder.rewards_token_mint
        );
        invariant!(self.claim_fee_token_account.delegate.is_none());
        invariant!(self.claim_fee_token_account.close_authority.is_none());

        assert_keys_eq!(
            self.fee_to_token_account.mint,
            self.rewarder.rewards_token_mint,
            "fee_to_token_account.mint"
        );
        assert_keys_eq!(
            self.fee_to_token_account.owner,
            addresses::FEE_TO,
            "fee_to_token_account.owner"
        );

        assert_keys_eq!(self.fee_to_token_account.owner, addresses::FEE_TO);
        assert_keys_eq!(
            self.fee_to_token_account.mint,
            self.rewarder.rewards_token_mint
        );
        invariant!(self.fee_to_token_account.delegate.is_none());
        invariant!(self.fee_to_token_account.close_authority.is_none());

        Ok(())
    }
}

impl<'info> Validate<'info> for MutableRewarderWithAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.rewarder.authority, self.authority, "authority");
        Ok(())
    }
}

impl<'info> Validate<'info> for ReadOnlyRewarderWithAuthority<'info> {
    /// Validates the [crate::Rewarder] is correct.
    fn validate(&self) -> Result<()> {
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.rewarder.authority);
        Ok(())
    }
}
