//! Validations for various accounts.

use anchor_lang::prelude::*;
use vipers::prelude::*;

use crate::addresses;
use crate::{
    AcceptAuthority, ExtractFees, MutableRewarderWithAuthority, MutableRewarderWithPauseAuthority,
    ReadOnlyRewarderWithAuthority, SetAnnualRewards, SetFamine, SetPauseAuthority, SetRewardsShare,
    TransferAuthority, UpdateQuarryRewards, UserStake,
};

// --------------------------------
// Rewarder Functions
// --------------------------------

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
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.rewarder.authority, Unauthorized);
        Ok(())
    }
}

impl<'info> Validate<'info> for AcceptAuthority<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;
        invariant!(
            self.rewarder.pending_authority != Pubkey::default(),
            PendingAuthorityNotSet
        );
        assert_keys_eq!(
            self.rewarder.pending_authority,
            self.authority,
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for SetAnnualRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.rewarder.assert_not_paused()?;
        self.auth.validate()?;
        Ok(())
    }
}

// --------------------------------
// Quarry functions
// --------------------------------

impl<'info> Validate<'info> for SetRewardsShare<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.quarry.rewarder, self.auth.rewarder);
        self.auth.rewarder.assert_not_paused()?;
        self.auth.validate()?;
        Ok(())
    }
}

impl<'info> Validate<'info> for SetFamine<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.quarry.rewarder, self.auth.rewarder);
        self.auth.rewarder.assert_not_paused()?;
        self.auth.validate()?;
        Ok(())
    }
}

impl<'info> Validate<'info> for UpdateQuarryRewards<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);
        self.rewarder.assert_not_paused()?;
        Ok(())
    }
}

/// --------------------------------
/// Miner functions
/// --------------------------------

impl<'info> Validate<'info> for UserStake<'info> {
    /// Validates the UserStake.
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;

        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority);

        // quarry
        assert_keys_eq!(self.miner.quarry, self.quarry);

        // miner_vault
        let staked_mint = self.quarry.token_mint_key;
        assert_keys_eq!(self.miner.token_vault_key, self.miner_vault);
        assert_keys_eq!(self.miner_vault.mint, staked_mint);
        assert_keys_eq!(self.miner_vault.owner, self.miner);

        // token_account
        assert_keys_eq!(self.token_account.mint, staked_mint);

        // rewarder
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        Ok(())
    }
}

impl<'info> Validate<'info> for ExtractFees<'info> {
    fn validate(&self) -> Result<()> {
        self.rewarder.assert_not_paused()?;

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
            self.rewarder.rewards_token_mint
        );
        assert_keys_eq!(self.fee_to_token_account.owner, addresses::FEE_TO);

        assert_keys_eq!(
            self.fee_to_token_account.mint,
            self.rewarder.rewards_token_mint
        );
        invariant!(self.fee_to_token_account.delegate.is_none());
        invariant!(self.fee_to_token_account.close_authority.is_none());

        assert_keys_neq!(self.claim_fee_token_account, self.fee_to_token_account);

        Ok(())
    }
}

impl<'info> Validate<'info> for MutableRewarderWithAuthority<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.rewarder.authority, self.authority, Unauthorized);
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
