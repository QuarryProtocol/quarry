use crate::{
    CreateOperator, DelegateCreateQuarry, DelegateSetAnnualRewards, DelegateSetFamine,
    DelegateSetRewardsShare, SetRole, WithDelegate,
};
use anchor_lang::prelude::*;
use vipers::{assert_keys_eq, validate::Validate};

impl<'info> Validate<'info> for CreateOperator<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRole<'info> {
    fn validate(&self) -> ProgramResult {
        assert_keys_eq!(self.operator.admin, self.admin, Unauthorized);
        Ok(())
    }
}

impl<'info> Validate<'info> for WithDelegate<'info> {
    fn validate(&self) -> ProgramResult {
        assert_keys_eq!(self.operator.rewarder, self.rewarder, "operator.rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetAnnualRewards<'info> {
    fn validate(&self) -> ProgramResult {
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
    fn validate(&self) -> ProgramResult {
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
    fn validate(&self) -> ProgramResult {
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
    fn validate(&self) -> ProgramResult {
        self.with_delegate.validate()?;
        assert_keys_eq!(
            self.with_delegate.operator.rate_setter,
            self.with_delegate.delegate,
            Unauthorized
        );
        Ok(())
    }
}
