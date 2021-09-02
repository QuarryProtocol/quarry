//! Validations for various accounts.

use anchor_lang::prelude::*;
use vipers::validate::Validate;
use vipers::{assert_keys, assert_owner, assert_program};

use crate::{NewRegistry, SyncQuarry};

impl<'info> Validate<'info> for NewRegistry<'info> {
    fn validate(&self) -> ProgramResult {
        assert_owner!(self.rewarder, quarry_mine::ID);
        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        Ok(())
    }
}

impl<'info> Validate<'info> for SyncQuarry<'info> {
    fn validate(&self) -> ProgramResult {
        assert_owner!(self.quarry, quarry_mine::ID);
        assert_keys!(
            self.quarry.rewarder_key,
            self.registry.rewarder,
            "quarry.rewarder_key"
        );
        Ok(())
    }
}
