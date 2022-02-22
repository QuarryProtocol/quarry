//! Validators for mint wrapper accounts.

use anchor_lang::prelude::*;
use vipers::prelude::*;

use crate::AcceptAdmin;
use crate::MinterUpdate;
use crate::NewMinter;
use crate::NewWrapper;
use crate::OnlyAdmin;
use crate::PerformMint;
use crate::TransferAdmin;

/// --------------------------------
/// Instruction account structs
/// --------------------------------

impl<'info> Validate<'info> for NewWrapper<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.token_mint.mint_authority.unwrap(),
            self.mint_wrapper,
            "mint authority"
        );
        assert_keys_eq!(
            self.token_mint.freeze_authority.unwrap(),
            self.mint_wrapper,
            "freeze authority"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for NewMinter<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        Ok(())
    }
}

impl<'info> Validate<'info> for MinterUpdate<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        assert_keys_eq!(
            self.minter.mint_wrapper,
            self.auth.mint_wrapper,
            "mint_wrapper"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAdmin<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.admin.is_signer, Unauthorized);
        assert_keys_eq!(self.admin, self.mint_wrapper.admin, "admin");
        assert_keys_neq!(self.next_admin, self.mint_wrapper.admin, "next_admin");

        Ok(())
    }
}

impl<'info> Validate<'info> for AcceptAdmin<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.pending_admin.is_signer, Unauthorized);
        assert_keys_eq!(
            self.pending_admin,
            self.mint_wrapper.pending_admin,
            "pending admin"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for PerformMint<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(
            self.mint_wrapper.to_account_info().is_writable,
            Unauthorized
        );
        invariant!(self.minter.to_account_info().is_writable, Unauthorized);

        invariant!(self.minter_authority.is_signer, Unauthorized);
        invariant!(self.minter.allowance > 0, MinterAllowanceExceeded);
        assert_keys_eq!(self.minter.mint_wrapper, self.mint_wrapper, "mint wrapper");
        assert_keys_eq!(
            self.minter_authority,
            self.minter.minter_authority,
            "minter"
        );
        assert_keys_eq!(self.token_mint, self.mint_wrapper.token_mint, "token mint");
        assert_keys_eq!(self.destination.mint, self.token_mint, "dest token mint");
        Ok(())
    }
}

/// --------------------------------
/// Account Structs
/// --------------------------------

impl<'info> Validate<'info> for OnlyAdmin<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.admin.is_signer, Unauthorized);
        assert_keys_eq!(self.admin, self.mint_wrapper.admin, "admin");
        Ok(())
    }
}
