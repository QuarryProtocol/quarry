//! Liquidity mining rewards distribution program.
//!
//! The program consists of three types of accounts:
//!
//! - [Rewarder], which controls token rewards distribution
//! - [Quarry], which receive rewards, and
//! - [Miner], which stake tokens into [Quarry]s to receive rewards.
//!
//! This program is modeled after [Synthetix's StakingRewards.sol](https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol).
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

#[macro_use]
mod macros;
mod state;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer};
use payroll::Payroll;
pub use state::*;
use std::cmp;
use vipers::prelude::*;

pub mod account_validators;
pub mod addresses;
pub mod payroll;
pub mod quarry;
pub mod rewarder;

mod instructions;
pub use instructions::*;

use crate::quarry::StakeAction;

declare_id!("QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB");

/// Maximum number of tokens that can be rewarded by a [Rewarder] per year.
pub const MAX_ANNUAL_REWARDS_RATE: u64 = u64::MAX >> 3;

/// The fees of new [Rewarder]s: 1,000 milliBPS = 1 BP or 0.01%.
/// This may be changed by governance in the future via program upgrade.
pub const DEFAULT_CLAIM_FEE_MILLIBPS: u64 = 1_000;

/// The maximum number of basis points possible.
pub const MAX_BPS: u64 = 10_000;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Quarry Mine",
    project_url: "https://quarry.so",
    contacts: "email:team@quarry.so",
    policy: "https://github.com/QuarryProtocol/quarry/blob/master/SECURITY.md",

    source_code: "https://github.com/QuarryProtocol/quarry",
    auditors: "Quantstamp"
}

/// Program for [quarry_mine].
#[program]
pub mod quarry_mine {
    use super::*;

    // --------------------------------
    // Rewarder Functions
    // --------------------------------

    /// Creates a new [Rewarder].
    #[deprecated(since = "5.0.0", note = "Use `new_rewarder_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn new_rewarder(ctx: Context<NewRewarder>, _bump: u8) -> Result<()> {
        instructions::new_rewarder::handler(ctx)
    }

    /// Creates a new [Rewarder].
    ///
    /// The V2 variant removes the need for supplying the bump and clock.
    #[access_control(ctx.accounts.validate())]
    pub fn new_rewarder_v2(ctx: Context<NewRewarderV2>) -> Result<()> {
        instructions::new_rewarder_v2::handler(ctx)
    }

    /// Sets the pause authority.
    #[access_control(ctx.accounts.validate())]
    pub fn set_pause_authority(ctx: Context<SetPauseAuthority>) -> Result<()> {
        let rewarder = &mut ctx.accounts.auth.rewarder;
        rewarder.pause_authority = ctx.accounts.new_pause_authority.key();
        Ok(())
    }

    /// Pauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn pause(ctx: Context<MutableRewarderWithPauseAuthority>) -> Result<()> {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.is_paused = true;
        Ok(())
    }

    /// Unpauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn unpause(ctx: Context<MutableRewarderWithPauseAuthority>) -> Result<()> {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.is_paused = false;
        Ok(())
    }

    /// Transfers the [Rewarder] authority to a different account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.pending_authority = new_authority;
        Ok(())
    }

    /// Accepts the authority to become the new rewarder.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_authority(ctx: Context<AcceptAuthority>) -> Result<()> {
        let rewarder = &mut ctx.accounts.rewarder;
        let next_authority = rewarder.pending_authority;
        rewarder.authority = next_authority;
        rewarder.pending_authority = Pubkey::default();
        Ok(())
    }

    /// Sets the amount of reward tokens distributed to all [Quarry]s per day.
    #[access_control(ctx.accounts.validate())]
    pub fn set_annual_rewards(ctx: Context<SetAnnualRewards>, new_rate: u64) -> Result<()> {
        invariant!(
            new_rate <= MAX_ANNUAL_REWARDS_RATE,
            MaxAnnualRewardsRateExceeded
        );
        let rewarder = &mut ctx.accounts.auth.rewarder;
        let previous_rate = rewarder.annual_rewards_rate;
        rewarder.annual_rewards_rate = new_rate;

        let current_ts = Clock::get()?.unix_timestamp;
        emit!(RewarderAnnualRewardsUpdateEvent {
            previous_rate,
            new_rate,
            timestamp: current_ts,
        });

        Ok(())
    }

    // --------------------------------
    // Quarry functions
    // --------------------------------

    /// Creates a new [Quarry].
    /// This may only be called by the [Rewarder]::authority.
    #[deprecated(since = "5.0.0", note = "Use `create_quarry_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn create_quarry(ctx: Context<CreateQuarry>, _bump: u8) -> Result<()> {
        instructions::create_quarry::handler(ctx)
    }

    /// Creates a new [Quarry].
    /// This may only be called by the [Rewarder]::authority.
    ///
    /// The V2 variant removes the need for supplying the bump and clock.
    #[access_control(ctx.accounts.validate())]
    pub fn create_quarry_v2(ctx: Context<CreateQuarryV2>) -> Result<()> {
        instructions::create_quarry_v2::handler(ctx)
    }

    /// Sets the rewards share of a quarry.
    #[access_control(ctx.accounts.validate())]
    pub fn set_rewards_share(ctx: Context<SetRewardsShare>, new_share: u64) -> Result<()> {
        let rewarder = &mut ctx.accounts.auth.rewarder;
        let quarry = &mut ctx.accounts.quarry;
        rewarder.total_rewards_shares = unwrap_int!(rewarder
            .total_rewards_shares
            .checked_add(new_share)
            .and_then(|v| v.checked_sub(quarry.rewards_share)));

        let now = Clock::get()?.unix_timestamp;
        quarry.last_update_ts = cmp::min(now, quarry.famine_ts);
        quarry.annual_rewards_rate = rewarder.compute_quarry_annual_rewards_rate(new_share)?;
        quarry.rewards_share = new_share;

        emit!(QuarryRewardsUpdateEvent {
            token_mint: quarry.token_mint_key,
            annual_rewards_rate: quarry.annual_rewards_rate,
            rewards_share: quarry.rewards_share,
            timestamp: now,
        });

        Ok(())
    }

    /// Sets the famine, which stops rewards.
    #[access_control(ctx.accounts.validate())]
    pub fn set_famine(ctx: Context<SetFamine>, famine_ts: i64) -> Result<()> {
        let quarry = &mut ctx.accounts.quarry;
        quarry.famine_ts = famine_ts;

        Ok(())
    }

    /// Synchronizes quarry rewards with the rewarder.
    /// Anyone can call this.
    #[access_control(ctx.accounts.validate())]
    pub fn update_quarry_rewards(ctx: Context<UpdateQuarryRewards>) -> Result<()> {
        let current_ts = Clock::get()?.unix_timestamp;
        let rewarder = &ctx.accounts.rewarder;
        let payroll: Payroll = (*ctx.accounts.quarry).into();
        let quarry = &mut ctx.accounts.quarry;
        quarry.update_rewards_internal(current_ts, rewarder, &payroll)?;

        emit!(QuarryRewardsUpdateEvent {
            token_mint: quarry.token_mint_key,
            annual_rewards_rate: quarry.annual_rewards_rate,
            rewards_share: quarry.rewards_share,
            timestamp: current_ts,
        });

        Ok(())
    }

    /// --------------------------------
    /// Miner functions
    /// --------------------------------

    /// Creates a [Miner] for the given authority.
    ///
    /// Anyone can call this; this is an associated account.
    #[deprecated(since = "5.0.0", note = "Use `create_miner_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn create_miner(ctx: Context<CreateMiner>, _bump: u8) -> Result<()> {
        instructions::create_miner::handler(ctx)
    }

    /// Creates a [Miner] for the given authority.
    ///
    /// Anyone can call this; this is an associated account.
    ///
    /// The V2 variant removes the need for supplying the bump.
    #[access_control(ctx.accounts.validate())]
    pub fn create_miner_v2(ctx: Context<CreateMiner>) -> Result<()> {
        instructions::create_miner::handler(ctx)
    }

    /// Claims rewards for the [Miner].
    #[deprecated(since = "5.0.0", note = "Use `claim_rewards_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::claim_rewards::handler(ctx)
    }

    /// Claims rewards for the [Miner].
    ///
    /// The V2 variant removes 2 accounts from the [crate::quarry_mine::claim_rewards] instruction.
    #[access_control(ctx.accounts.validate())]
    pub fn claim_rewards_v2(ctx: Context<ClaimRewardsV2>) -> Result<()> {
        instructions::claim_rewards_v2::handler(ctx)
    }

    /// Stakes tokens into the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn stake_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
        if amount == 0 {
            // noop
            return Ok(());
        }

        let quarry = &mut ctx.accounts.quarry;
        let clock = Clock::get()?;
        quarry.process_stake_action_internal(
            StakeAction::Stake,
            clock.unix_timestamp,
            &ctx.accounts.rewarder,
            &mut ctx.accounts.miner,
            amount,
        )?;

        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.miner_vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        // Transfer LP tokens to quarry vault
        token::transfer(cpi_context, amount)?;

        emit!(StakeEvent {
            timestamp: clock.unix_timestamp,
            authority: ctx.accounts.authority.key(),
            amount,
            token: ctx.accounts.token_account.mint,
        });
        Ok(())
    }

    /// Withdraws tokens from the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn withdraw_tokens(ctx: Context<UserStake>, amount: u64) -> Result<()> {
        if amount == 0 {
            // noop
            return Ok(());
        }
        invariant!(
            amount <= ctx.accounts.miner_vault.amount,
            InsufficientBalance
        );

        let clock = Clock::get()?;
        let quarry = &mut ctx.accounts.quarry;
        quarry.process_stake_action_internal(
            StakeAction::Withdraw,
            clock.unix_timestamp,
            &ctx.accounts.rewarder,
            &mut ctx.accounts.miner,
            amount,
        )?;

        // Sign a transfer instruction as the [Miner]
        let miner_seeds = &[
            b"Miner".as_ref(),
            ctx.accounts.miner.quarry.as_ref(),
            ctx.accounts.miner.authority.as_ref(),
            &[ctx.accounts.miner.bump],
        ];
        let signer_seeds = &[&miner_seeds[..]];
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.miner_vault.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.miner.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        // Transfer out LP tokens from quarry vault
        token::transfer(cpi_ctx, amount)?;

        emit!(WithdrawEvent {
            timestamp: clock.unix_timestamp,
            authority: ctx.accounts.authority.key(),
            amount,
            token: ctx.accounts.token_account.mint,
        });
        Ok(())
    }

    /// Withdraw tokens from a [Miner]-owned token account that is not the [Miner::token_vault_key].
    /// This is useful for if tokens are sent directly to a [Miner].
    ///
    /// Only the [Miner::authority] may call this.
    #[access_control(ctx.accounts.validate())]
    pub fn rescue_tokens(ctx: Context<RescueTokens>) -> Result<()> {
        instructions::rescue_tokens::handler(ctx)
    }

    // --------------------------------
    // Protocol Functions
    // --------------------------------

    /// Extracts fees to the Quarry DAO.
    /// This can be called by anyone.
    #[access_control(ctx.accounts.validate())]
    pub fn extract_fees(ctx: Context<ExtractFees>) -> Result<()> {
        let seeds = gen_rewarder_signer_seeds!(ctx.accounts.rewarder);
        let signer_seeds = &[&seeds[..]];

        // Transfer the tokens to the DAO address.
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.claim_fee_token_account.to_account_info(),
                    to: ctx.accounts.fee_to_token_account.to_account_info(),
                    authority: ctx.accounts.rewarder.to_account_info(),
                },
                signer_seeds,
            ),
            ctx.accounts.claim_fee_token_account.amount,
        )?;

        Ok(())
    }
}

/// --------------------------------
/// Context Structs
/// --------------------------------

/* Rewarder contexts */

/// Accounts for [quarry_mine::set_pause_authority].
#[derive(Accounts)]
pub struct SetPauseAuthority<'info> {
    /// [Rewarder].
    pub auth: MutableRewarderWithAuthority<'info>,

    /// The pause authority.
    /// CHECK: OK
    pub new_pause_authority: UncheckedAccount<'info>,
}

/// Accounts for [quarry_mine::transfer_authority].
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

/// Accounts for [quarry_mine::accept_authority].
#[derive(Accounts)]
pub struct AcceptAuthority<'info> {
    /// Authority of the next rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

/// Mutable [Rewarder] that requires the authority to be a signer.
#[derive(Accounts, Clone)]
pub struct MutableRewarderWithAuthority<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut, has_one = authority @ ErrorCode::Unauthorized)]
    pub rewarder: Account<'info, Rewarder>,
}

/// Read-only [Rewarder] that requires the authority to be a signer.
#[derive(Accounts)]
pub struct ReadOnlyRewarderWithAuthority<'info> {
    /// Authority of the rewarder.
    pub authority: Signer<'info>,

    /// [Rewarder].
    #[account(has_one = authority)]
    pub rewarder: Account<'info, Rewarder>,
}

/// Accounts for [quarry_mine::set_annual_rewards].
#[derive(Accounts)]
pub struct SetAnnualRewards<'info> {
    /// [Rewarder],
    pub auth: MutableRewarderWithAuthority<'info>,
}

/* Quarry contexts */

/// Accounts for [quarry_mine::set_famine].
#[derive(Accounts)]
pub struct SetFamine<'info> {
    /// [Rewarder] of the [Quarry].
    pub auth: ReadOnlyRewarderWithAuthority<'info>,

    /// [Quarry] updated.
    #[account(mut, constraint = quarry.rewarder == auth.rewarder.key())]
    pub quarry: Account<'info, Quarry>,
}

/// Accounts for [quarry_mine::set_rewards_share].
#[derive(Accounts)]
pub struct SetRewardsShare<'info> {
    /// [Rewarder] of the [Quarry].
    pub auth: MutableRewarderWithAuthority<'info>,

    /// [Quarry] updated.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,
}

/// Accounts for [quarry_mine::update_quarry_rewards].
#[derive(Accounts)]
pub struct UpdateQuarryRewards<'info> {
    /// [Quarry].
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,

    /// [Rewarder].
    pub rewarder: Account<'info, Rewarder>,
}

/* Miner contexts */

/// Staking accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
#[derive(Accounts, Clone)]
pub struct UserStake<'info> {
    /// Miner authority (i.e. the user).
    pub authority: Signer<'info>,

    /// Miner.
    #[account(mut)]
    pub miner: Account<'info, Miner>,

    /// Quarry to claim from.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,

    /// Vault of the miner.
    #[account(mut)]
    pub miner_vault: Account<'info, TokenAccount>,

    /// User's staked token account
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rewarder
    pub rewarder: Account<'info, Rewarder>,
}

/// Accounts for [quarry_mine::extract_fees].
#[derive(Accounts)]
pub struct ExtractFees<'info> {
    /// Rewarder to extract fees from.
    #[account(has_one = claim_fee_token_account)]
    pub rewarder: Account<'info, Rewarder>,

    /// [TokenAccount] which receives claim fees.
    #[account(mut)]
    pub claim_fee_token_account: Account<'info, TokenAccount>,

    /// [TokenAccount] owned by the [addresses::FEE_TO].
    /// Holds DAO claim fees.
    #[account(mut)]
    pub fee_to_token_account: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

/// Accounts for [quarry_mine::pause] and [quarry_mine::unpause].
#[derive(Accounts)]
pub struct MutableRewarderWithPauseAuthority<'info> {
    /// Pause authority of the rewarder.
    pub pause_authority: Signer<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: Account<'info, Rewarder>,
}

/// --------------------------------
/// Events
/// --------------------------------

/// Emitted when reward tokens are claimed.
#[event]
pub struct ClaimEvent {
    /// Authority staking.
    #[index]
    pub authority: Pubkey,
    /// Token of the pool staked into.
    #[index]
    pub staked_token: Pubkey,
    /// Token received as rewards.
    pub rewards_token: Pubkey,
    /// Amount of rewards token received.
    pub amount: u64,
    /// Fees paid.
    pub fees: u64,
    /// When the event occurred.
    pub timestamp: i64,
}

/// Emitted when tokens are staked into a [Quarry].
#[event]
pub struct StakeEvent {
    /// Authority staking.
    #[index]
    pub authority: Pubkey,
    /// Mint of token staked.
    #[index]
    pub token: Pubkey,
    /// Amount staked.
    pub amount: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// Emitted when tokens are withdrawn from a [Quarry].
#[event]
pub struct WithdrawEvent {
    /// Authority withdrawing.
    #[index]
    pub authority: Pubkey,
    /// Mint of token withdrawn.
    #[index]
    pub token: Pubkey,
    /// Amount withdrawn.
    pub amount: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// Emitted when the daily rewards rate is updated.
#[event]
pub struct RewarderAnnualRewardsUpdateEvent {
    /// Previous rate of rewards.
    pub previous_rate: u64,
    /// New rate of rewards.
    pub new_rate: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// Emitted when a new quarry is created.
#[event]
pub struct QuarryCreateEvent {
    /// [Mint] of the [Quarry] token.
    pub token_mint: Pubkey,
    /// When the event took place.
    pub timestamp: i64,
}

/// Emitted when a quarry's reward share is updated.
#[event]
pub struct QuarryRewardsUpdateEvent {
    /// [Mint] of the [Quarry] token.
    pub token_mint: Pubkey,
    /// New annual rewards rate
    pub annual_rewards_rate: u64,
    /// New rewards share.
    pub rewards_share: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// --------------------------------
/// Error Codes
/// --------------------------------
#[error_code]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("Insufficient staked balance for withdraw request.")]
    InsufficientBalance,
    #[msg("Pending authority not set")]
    PendingAuthorityNotSet,
    #[msg("Invalid quarry rewards share")]
    InvalidRewardsShare,
    #[msg("Insufficient allowance.")]
    InsufficientAllowance,
    #[msg("New vault not empty.")]
    NewVaultNotEmpty,
    #[msg("Not enough tokens.")]
    NotEnoughTokens,
    #[msg("Invalid timestamp.")]
    InvalidTimestamp,
    #[msg("Invalid max claim fee.")]
    InvalidMaxClaimFee,
    #[msg("Max annual rewards rate exceeded.")]
    MaxAnnualRewardsRateExceeded,
    #[msg("Rewarder is paused.")]
    Paused,
    #[msg("Rewards earned exceeded quarry's upper bound.")]
    UpperboundExceeded,
}
