//! Validations for various accounts.

use anchor_lang::prelude::*;
use vipers::{assert_keys_eq, validate::Validate};

use crate::{NewRegistry, SyncQuarry};

impl<'info> Validate<'info> for NewRegistry<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for SyncQuarry<'info> {
    fn validate(&self) -> ProgramResult {
        assert_keys_eq!(
            self.quarry.rewarder_key,
            self.registry.rewarder,
            "quarry.rewarder_key"
        );
        Ok(())
    }
}
