//! Validators for Quarry operator accounts.

use crate::{
    CreateOperator, DelegateCreateQuarry, DelegateSetAnnualRewards, DelegateSetFamine,
    DelegateSetRewardsShare, SetRole, WithDelegate,
};
use anchor_lang::prelude::*;
use vipers::prelude::*;

impl<'info> Validate<'info> for CreateOperator<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.operator,
            self.rewarder.pending_authority,
            PendingAuthorityNotSet
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRole<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.operator.admin, self.admin, Unauthorized);
        Ok(())
    }
}

impl<'info> Validate<'info> for WithDelegate<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.operator.rewarder, self.rewarder, "operator.rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetAnnualRewards<'info> {
    fn validate(&self) -> Result<()> {
        self.with_delegate.validate()?;
        assert_keys_eq!(
            self.with_delegate.operator.rate_setter,
            self.with_delegate.delegate,
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateCreateQuarry<'info> {
    fn validate(&self) -> Result<()> {
        self.with_delegate.validate()?;
        assert_keys_eq!(
            self.with_delegate.operator.quarry_creator,
            self.with_delegate.delegate,
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetRewardsShare<'info> {
    fn validate(&self) -> Result<()> {
        self.with_delegate.validate()?;
        assert_keys_eq!(
            self.with_delegate.operator.share_allocator,
            self.with_delegate.delegate,
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetFamine<'info> {
    fn validate(&self) -> Result<()> {
        self.with_delegate.validate()?;
        assert_keys_eq!(
            self.with_delegate.operator.share_allocator,
            self.with_delegate.delegate,
            Unauthorized
        );
        Ok(())
    }
}
