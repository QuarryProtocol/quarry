use anchor_lang::prelude::*;
use vipers::assert_keys;
use vipers::validate::Validate;

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
    fn validate(&self) -> ProgramResult {
        assert_keys!(
            self.token_mint.mint_authority.unwrap(),
            self.mint_wrapper,
            "mint authority"
        );
        assert_keys!(
            self.token_mint.freeze_authority.unwrap(),
            self.mint_wrapper,
            "freeze authority"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for NewMinter<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        Ok(())
    }
}

impl<'info> Validate<'info> for MinterUpdate<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        assert_keys!(
            self.minter.mint_wrapper,
            self.auth.mint_wrapper,
            "mint_wrapper"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAdmin<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.admin.is_signer, Unauthorized);
        assert_keys!(self.admin, self.mint_wrapper.admin, "admin");
        Ok(())
    }
}

impl<'info> Validate<'info> for AcceptAdmin<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.pending_admin.is_signer, Unauthorized);
        assert_keys!(
            self.pending_admin,
            self.mint_wrapper.pending_admin,
            "pending admin"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for PerformMint<'info> {
    fn validate(&self) -> ProgramResult {
        require!(
            self.mint_wrapper.to_account_info().is_writable,
            Unauthorized
        );
        require!(self.minter.to_account_info().is_writable, Unauthorized);

        require!(self.minter_authority.is_signer, Unauthorized);
        require!(self.minter.allowance > 0, MinterAllowanceExceeded);
        assert_keys!(self.minter.mint_wrapper, self.mint_wrapper, "mint wrapper");
        assert_keys!(
            self.minter_authority,
            self.minter.minter_authority,
            "minter"
        );
        assert_keys!(self.token_mint, self.mint_wrapper.token_mint, "token mint");
        assert_keys!(self.destination.mint, self.token_mint, "dest token mint");
        Ok(())
    }
}

/// --------------------------------
/// Account Structs
/// --------------------------------

impl<'info> Validate<'info> for OnlyAdmin<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.admin.is_signer, Unauthorized);
        assert_keys!(self.admin, self.mint_wrapper.admin, "admin");
        Ok(())
    }
}
