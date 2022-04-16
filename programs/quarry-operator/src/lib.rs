//! Delegates Quarry Rewarder authority roles.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use quarry_mine::{Quarry, Rewarder};
use vipers::prelude::*;

mod account_validators;
mod instructions;
mod macros;
mod state;

use instructions::*;
pub use state::*;

declare_id!("QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Quarry Operator",
    project_url: "https://quarry.so",
    contacts: "email:team@quarry.so",
    policy: "https://github.com/QuarryProtocol/quarry/blob/master/SECURITY.md",

    source_code: "https://github.com/QuarryProtocol/quarry",
    auditors: "Quantstamp"
}

/// Quarry Operator program.
#[program]
pub mod quarry_operator {
    use super::*;

    /// Creates a new [Operator].
    #[deprecated(since = "5.0.0", note = "Use `create_operator_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn create_operator(ctx: Context<CreateOperator>, _bump: u8) -> Result<()> {
        instructions::create_operator::handler(ctx)
    }

    /// Creates a new [Operator].
    ///
    /// The V2 variant removes the need for supplying the bump.
    #[access_control(ctx.accounts.validate())]
    pub fn create_operator_v2(ctx: Context<CreateOperator>) -> Result<()> {
        instructions::create_operator::handler(ctx)
    }

    /// Sets the account that can set roles.
    #[access_control(ctx.accounts.validate())]
    pub fn set_admin(ctx: Context<SetRole>) -> Result<()> {
        let operator = &mut ctx.accounts.operator;
        operator.admin = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::set_annual_rewards].
    #[access_control(ctx.accounts.validate())]
    pub fn set_rate_setter(ctx: Context<SetRole>) -> Result<()> {
        let operator = &mut ctx.accounts.operator;
        operator.rate_setter = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::create_quarry].
    #[access_control(ctx.accounts.validate())]
    pub fn set_quarry_creator(ctx: Context<SetRole>) -> Result<()> {
        let operator = &mut ctx.accounts.operator;
        operator.quarry_creator = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::set_rewards_share].
    #[access_control(ctx.accounts.validate())]
    pub fn set_share_allocator(ctx: Context<SetRole>) -> Result<()> {
        let operator = &mut ctx.accounts.operator;
        operator.share_allocator = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Calls [quarry_mine::quarry_mine::set_annual_rewards].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_set_annual_rewards(
        ctx: Context<DelegateSetAnnualRewards>,
        new_rate: u64,
    ) -> Result<()> {
        let operator = &ctx.accounts.with_delegate.operator;
        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
        quarry_mine::cpi::set_annual_rewards(
            CpiContext::new_with_signer(
                ctx.accounts
                    .with_delegate
                    .quarry_mine_program
                    .to_account_info(),
                quarry_mine::cpi::accounts::SetAnnualRewards {
                    auth: ctx.accounts.with_delegate.to_auth_accounts(),
                },
                signer_seeds,
            ),
            new_rate,
        )?;
        Ok(())
    }

    /// Calls [quarry_mine::quarry_mine::create_quarry_v2].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_create_quarry(ctx: Context<DelegateCreateQuarry>, _bump: u8) -> Result<()> {
        instructions::delegate_create_quarry::handler(ctx)
    }

    /// Calls [quarry_mine::quarry_mine::create_quarry_v2].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_create_quarry_v2(ctx: Context<DelegateCreateQuarryV2>) -> Result<()> {
        instructions::delegate_create_quarry_v2::handler(ctx)
    }

    /// Calls [quarry_mine::quarry_mine::set_rewards_share].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_set_rewards_share(
        ctx: Context<DelegateSetRewardsShare>,
        new_share: u64,
    ) -> Result<()> {
        let operator = &ctx.accounts.with_delegate.operator;
        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
        quarry_mine::cpi::set_rewards_share(
            CpiContext::new_with_signer(
                ctx.accounts
                    .with_delegate
                    .quarry_mine_program
                    .to_account_info(),
                quarry_mine::cpi::accounts::SetRewardsShare {
                    auth: ctx.accounts.with_delegate.to_auth_accounts(),
                    quarry: ctx.accounts.quarry.to_account_info(),
                },
                signer_seeds,
            ),
            new_share,
        )?;
        Ok(())
    }

    /// Calls [quarry_mine::quarry_mine::set_famine].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_set_famine(ctx: Context<DelegateSetFamine>, famine_ts: i64) -> Result<()> {
        let operator = &ctx.accounts.with_delegate.operator;
        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];

        quarry_mine::cpi::set_famine(
            CpiContext::new_with_signer(
                ctx.accounts
                    .with_delegate
                    .quarry_mine_program
                    .to_account_info(),
                quarry_mine::cpi::accounts::SetFamine {
                    auth: ctx.accounts.with_delegate.to_readonly_auth_accounts(),
                    quarry: ctx.accounts.quarry.to_account_info(),
                },
                signer_seeds,
            ),
            famine_ts,
        )
    }
}

// --------------------------------
// Instructions
// --------------------------------

/// Accounts for setting roles.
#[derive(Accounts)]
pub struct SetRole<'info> {
    /// The [Operator] of the [Rewarder].
    #[account(mut)]
    pub operator: Account<'info, Operator>,
    /// The [Operator::admin].
    pub admin: Signer<'info>,
    /// The account to give the role to.
    /// CHECK: Ok
    pub delegate: UncheckedAccount<'info>,
}

/// Accounts for [crate::quarry_operator::delegate_set_annual_rewards].
#[derive(Accounts)]
pub struct DelegateSetAnnualRewards<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
}

/// Accounts for [crate::quarry_operator::delegate_set_rewards_share].
#[derive(Accounts)]
pub struct DelegateSetRewardsShare<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
    /// [Quarry].
    #[account(
        mut,
        constraint = quarry.rewarder == with_delegate.rewarder.key()
    )]
    pub quarry: Account<'info, Quarry>,
}

/// Accounts for [crate::quarry_operator::delegate_set_famine].
#[derive(Accounts)]
pub struct DelegateSetFamine<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
    /// [Quarry].
    #[account(
        mut,
        constraint = quarry.rewarder == with_delegate.rewarder.key()
    )]
    pub quarry: Account<'info, Quarry>,
}

/// Accounts struct for instructions that must be signed by one of the delegates on the [Operator].
#[derive(Accounts, Clone)]
pub struct WithDelegate<'info> {
    /// The [Operator] of the [Rewarder].
    #[account(mut, has_one = rewarder)]
    pub operator: Account<'info, Operator>,
    /// The delegated account in one of the [Operator] roles.
    pub delegate: Signer<'info>,
    /// The [Rewarder].
    #[account(
        mut,
        constraint = rewarder.authority == operator.key() @ ErrorCode::OperatorNotRewarderAuthority
    )]
    pub rewarder: Account<'info, Rewarder>,
    /// Quarry mine
    pub quarry_mine_program: Program<'info, quarry_mine::program::QuarryMine>,
}

impl<'info> WithDelegate<'info> {
    /// Creates the [quarry_mine::cpi::accounts::MutableRewarderWithAuthority] accounts.
    pub fn to_auth_accounts(
        &self,
    ) -> quarry_mine::cpi::accounts::MutableRewarderWithAuthority<'info> {
        quarry_mine::cpi::accounts::MutableRewarderWithAuthority {
            authority: self.operator.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
        }
    }

    /// Creates the [quarry_mine::cpi::accounts::MutableRewarderWithAuthority] accounts.
    pub fn to_readonly_auth_accounts(
        &self,
    ) -> quarry_mine::cpi::accounts::ReadOnlyRewarderWithAuthority<'info> {
        quarry_mine::cpi::accounts::ReadOnlyRewarderWithAuthority {
            authority: self.operator.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
        }
    }
}

/// Errors
#[error_code]
pub enum ErrorCode {
    #[msg("Signer is not authorized to perform this action.")]
    Unauthorized,
    #[msg("Pending authority must be set to the created operator.")]
    PendingAuthorityNotSet,
    #[msg("Operator is not the Rewarder authority.")]
    OperatorNotRewarderAuthority,
}
