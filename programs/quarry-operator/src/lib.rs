//! Delegates Quarry Rewarder authority roles.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

use anchor_lang::prelude::*;
use quarry_mine::{Quarry, Rewarder};
use vipers::unwrap_int;
use vipers::validate::Validate;

mod account_validators;
mod macros;

declare_id!("QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz");

/// Quarry Operator program.
#[program]
pub mod quarry_operator {
    use super::*;

    /// Creates a new [Operator].
    #[access_control(ctx.accounts.validate())]
    pub fn create_operator(ctx: Context<CreateOperator>, bump: u8) -> ProgramResult {
        let operator = &mut ctx.accounts.operator;
        operator.base = ctx.accounts.base.key();
        operator.bump = bump;

        operator.rewarder = ctx.accounts.rewarder.key();
        operator.admin = ctx.accounts.admin.key();

        operator.rate_setter = operator.admin;
        operator.quarry_creator = operator.admin;
        operator.share_allocator = operator.admin;
        operator.record_update()?;

        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
        quarry_mine::cpi::accept_authority(CpiContext::new_with_signer(
            ctx.accounts.quarry_mine_program.to_account_info(),
            quarry_mine::cpi::accounts::AcceptAuthority {
                authority: ctx.accounts.operator.to_account_info(),
                rewarder: ctx.accounts.rewarder.to_account_info(),
            },
            signer_seeds,
        ))?;

        Ok(())
    }

    /// Sets the account that can set roles.
    #[access_control(ctx.accounts.validate())]
    pub fn set_admin(ctx: Context<SetRole>) -> ProgramResult {
        let operator = &mut ctx.accounts.operator;
        operator.admin = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::set_annual_rewards].
    #[access_control(ctx.accounts.validate())]
    pub fn set_rate_setter(ctx: Context<SetRole>) -> ProgramResult {
        let operator = &mut ctx.accounts.operator;
        operator.rate_setter = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::create_quarry].
    #[access_control(ctx.accounts.validate())]
    pub fn set_quarry_creator(ctx: Context<SetRole>) -> ProgramResult {
        let operator = &mut ctx.accounts.operator;
        operator.quarry_creator = ctx.accounts.delegate.key();
        operator.record_update()?;
        Ok(())
    }

    /// Sets who can call [quarry_mine::quarry_mine::set_rewards_share].
    #[access_control(ctx.accounts.validate())]
    pub fn set_share_allocator(ctx: Context<SetRole>) -> ProgramResult {
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
    ) -> ProgramResult {
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

    /// Calls [quarry_mine::quarry_mine::create_quarry].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_create_quarry(ctx: Context<DelegateCreateQuarry>, bump: u8) -> ProgramResult {
        let operator = &ctx.accounts.with_delegate.operator;
        let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
        quarry_mine::cpi::create_quarry(
            CpiContext::new_with_signer(
                ctx.accounts
                    .with_delegate
                    .quarry_mine_program
                    .to_account_info(),
                quarry_mine::cpi::accounts::CreateQuarry {
                    quarry: ctx.accounts.quarry.to_account_info(),
                    auth: ctx.accounts.with_delegate.to_auth_accounts(),
                    token_mint: ctx.accounts.token_mint.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    unused_clock: ctx.accounts.unused_clock.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                signer_seeds,
            ),
            bump,
        )?;
        Ok(())
    }

    /// Calls [quarry_mine::quarry_mine::set_rewards_share].
    #[access_control(ctx.accounts.validate())]
    pub fn delegate_set_rewards_share(
        ctx: Context<DelegateSetRewardsShare>,
        new_share: u64,
    ) -> ProgramResult {
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
    pub fn delegate_set_famine(ctx: Context<DelegateSetFamine>, famine_ts: i64) -> ProgramResult {
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

impl Operator {
    fn record_update(&mut self) -> ProgramResult {
        self.last_modified_ts = Clock::get()?.unix_timestamp;
        self.generation = unwrap_int!(self.generation.checked_add(1));
        Ok(())
    }
}

// --------------------------------
// Accounts
// --------------------------------

/// Operator state
#[account]
#[derive(Copy, Default, Debug, PartialEq, Eq)]
pub struct Operator {
    /// The base.
    pub base: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// The [Rewarder].
    pub rewarder: Pubkey,
    /// Can modify the authorities below.
    pub admin: Pubkey,

    /// Can call [quarry_mine::quarry_mine::set_annual_rewards].
    pub rate_setter: Pubkey,
    /// Can call [quarry_mine::quarry_mine::create_quarry].
    pub quarry_creator: Pubkey,
    /// Can call [quarry_mine::quarry_mine::set_rewards_share].
    pub share_allocator: Pubkey,

    /// When the [Operator] was last modified.
    pub last_modified_ts: i64,
    /// Auto-incrementing sequence number of the set of authorities.
    /// Useful for checking if things were updated.
    pub generation: u64,
}

// --------------------------------
// Instructions
// --------------------------------

/// Accounts for [crate::quarry_operator::create_operator].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateOperator<'info> {
    /// Base key used to create the [Operator].
    pub base: Signer<'info>,
    /// Operator PDA.
    #[account(
        init,
        seeds = [
            b"Operator".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub operator: Account<'info, Operator>,
    /// [Rewarder] of the token.
    #[account(mut)]
    pub rewarder: Box<Account<'info, Rewarder>>,
    /// The admin to set.
    pub admin: UncheckedAccount<'info>,
    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,
    /// [System] program.
    pub system_program: Program<'info, System>,
    /// Quarry mine
    pub quarry_mine_program: Program<'info, quarry_mine::program::QuarryMine>,
}

/// Accounts for setting roles.
#[derive(Accounts)]
pub struct SetRole<'info> {
    /// The [Operator] of the [Rewarder].
    #[account(mut)]
    pub operator: Account<'info, Operator>,
    /// The [Operator::admin].
    pub admin: Signer<'info>,
    /// The account to give the role to.
    pub delegate: UncheckedAccount<'info>,
}

/// Accounts for [crate::quarry_operator::delegate_set_annual_rewards].
#[derive(Accounts)]
pub struct DelegateSetAnnualRewards<'info> {
    pub with_delegate: WithDelegate<'info>,
}

/// Accounts for [crate::quarry_operator::delegate_create_quarry].
#[derive(Accounts)]
pub struct DelegateCreateQuarry<'info> {
    pub with_delegate: WithDelegate<'info>,
    #[account(mut)]
    pub quarry: UncheckedAccount<'info>,
    pub token_mint: Box<Account<'info, anchor_spl::token::Mint>>,

    /// Payer of [Quarry] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Unused variable that held the clock. Placeholder.
    pub unused_clock: UncheckedAccount<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

/// Accounts for [crate::quarry_operator::delegate_set_rewards_share].
#[derive(Accounts)]
pub struct DelegateSetRewardsShare<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
    /// [Quarry].
    #[account(mut)]
    pub quarry: Box<Account<'info, Quarry>>,
}

/// Accounts for [crate::quarry_operator::delegate_set_famine].
#[derive(Accounts)]
pub struct DelegateSetFamine<'info> {
    /// Delegate accounts.
    pub with_delegate: WithDelegate<'info>,
    /// [Quarry].
    #[account(mut)]
    pub quarry: Box<Account<'info, Quarry>>,
}

/// Delegate accounts.
#[derive(Accounts)]
pub struct WithDelegate<'info> {
    /// The [Operator] of the [Rewarder].
    #[account(mut)]
    pub operator: Account<'info, Operator>,
    /// The account to give the role to.
    pub delegate: Signer<'info>,
    /// The [Rewarder].
    #[account(mut)]
    pub rewarder: Box<Account<'info, Rewarder>>,
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
#[error]
pub enum ErrorCode {
    #[msg("Unauthorized.")]
    Unauthorized,
}
