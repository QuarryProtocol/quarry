use crate::{
    CreateOperator, DelegateCreateQuarry, DelegateSetAnnualRewards, DelegateSetRewardsShare,
    SetRole, WithDelegate,
};
use anchor_lang::prelude::*;
use vipers::{assert_keys, validate::Validate};

impl<'info> Validate<'info> for CreateOperator<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRole<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.operator.admin == self.admin.key(), Unauthorized);
        Ok(())
    }
}

impl<'info> Validate<'info> for WithDelegate<'info> {
    fn validate(&self) -> ProgramResult {
        assert_keys!(self.operator.rewarder, *self.rewarder, "operator.rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetAnnualRewards<'info> {
    fn validate(&self) -> ProgramResult {
        self.with_delegate.validate()?;
        require!(
            self.with_delegate.operator.rate_setter == self.with_delegate.delegate.key(),
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateCreateQuarry<'info> {
    fn validate(&self) -> ProgramResult {
        self.with_delegate.validate()?;
        require!(
            self.with_delegate.operator.quarry_creator == self.with_delegate.delegate.key(),
            Unauthorized
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for DelegateSetRewardsShare<'info> {
    fn validate(&self) -> ProgramResult {
        self.with_delegate.validate()?;
        require!(
            self.with_delegate.operator.share_allocator == self.with_delegate.delegate.key(),
            Unauthorized
        );
        Ok(())
    }
}
