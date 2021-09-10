use anchor_lang::prelude::*;
use anchor_spl::token;
use vipers::validate::Validate;
use vipers::{assert_keys, assert_owner, assert_program};

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

        assert_program!(self.token_program, TOKEN_PROGRAM_ID);
        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);

        assert_owner!(self.token_mint, token::ID);
        Ok(())
    }
}

impl<'info> Validate<'info> for NewMinter<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
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
        assert_program!(self.token_program, TOKEN_PROGRAM_ID);

        assert_owner!(self.token_mint, token::ID);
        assert_owner!(self.destination, token::ID);
        Ok(())
    }
}

/// --------------------------------
/// Account Structs
/// --------------------------------

impl<'info> Validate<'info> for OnlyAdmin<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.admin.is_signer, Unauthorized);
        require!(self.admin.is_writable, Unauthorized);
        assert_keys!(self.admin, self.mint_wrapper.admin, "admin");
        Ok(())
    }
}
