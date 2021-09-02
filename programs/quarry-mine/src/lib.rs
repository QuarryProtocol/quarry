//! Liquidity mining rewards distribution program.
//!
//! The program consists of three types of accounts:
//!
//! - [Rewarder], which controls token rewards distribution
//! - [Quarry], which receive rewards, and
//! - [Miner], which stake tokens into [Quarry]s to receive rewards.
//!
//! This program is modeled after [Synthetix's StakingRewards.sol](https://github.com/Synthetixio/synthetix/blob/4b9b2ee09b38638de6fe1c38dbe4255a11ebed86/contracts/StakingRewards.sol).

#[macro_use]
mod macros;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::declare_id;
use anchor_lang::Key;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer};
use num_traits::ToPrimitive;
use payroll::Payroll;
use std::cmp;
use vipers::assert_keys;
use vipers::unwrap_int;
use vipers::validate::Validate;

pub mod account_validators;
pub mod addresses;
pub mod payroll;
pub mod quarry;
pub mod rewarder;

use crate::quarry::StakeAction;

declare_id!("QMNFUvncKBh11ZgEwYtoup3aXvuVxt6fzrcsjk2cjpM");

/// Maximum number of tokens that can be rewarded by a [Rewarder] per year.
pub const MAX_ANNUAL_REWARDS_RATE: u64 = u64::MAX >> 3;

/// The fees of new [Rewarder]s-- 1,000 KBPS = 1 BP or 0.01%.
/// This may be changed by governance in the future via program upgrade.
pub const DEFAULT_CLAIM_FEE_KBPS: u64 = 1_000;

/// Program for [quarry_mine].
#[program]
pub mod quarry_mine {
    use super::*;

    /// --------------------------------
    /// Rewarder Functions
    /// --------------------------------

    /// Creates a new [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn new_rewarder(ctx: Context<NewRewarder>, bump: u8) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;

        rewarder.base = ctx.accounts.base.key();
        rewarder.bump = bump;

        rewarder.authority = ctx.accounts.authority.key();
        rewarder.pending_authority = Pubkey::default();

        rewarder.annual_rewards_rate = 0;
        rewarder.num_quarries = 0;
        rewarder.total_rewards_shares = 0;
        rewarder.mint_wrapper = ctx.accounts.mint_wrapper.key();

        rewarder.rewards_token_mint = ctx.accounts.rewards_token_mint.key();

        rewarder.claim_fee_token_account = ctx.accounts.claim_fee_token_account.key();
        rewarder.max_claim_fee_kbps = DEFAULT_CLAIM_FEE_KBPS;

        rewarder.pause_authority = Pubkey::default();
        rewarder.is_paused = false;

        emit!(NewRewarderEvent {
            authority: rewarder.authority,
            timestamp: ctx.accounts.clock.unix_timestamp,
        });

        Ok(())
    }

    /// Sets the pause authority.
    #[access_control(ctx.accounts.validate())]
    pub fn set_pause_authority(ctx: Context<SetPauseAuthority>) -> ProgramResult {
        let rewarder = &mut ctx.accounts.auth.rewarder;
        rewarder.pause_authority = ctx.accounts.pause_authority.key();
        Ok(())
    }

    /// Pauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn pause(ctx: Context<MutableRewarderWithPauseAuthority>) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.is_paused = true;
        Ok(())
    }

    /// Unpauses the [Rewarder].
    #[access_control(ctx.accounts.validate())]
    pub fn unpause(ctx: Context<MutableRewarderWithPauseAuthority>) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.is_paused = false;
        Ok(())
    }

    /// Transfers the [Rewarder] authority to a different account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;
        rewarder.pending_authority = new_authority;
        Ok(())
    }

    /// Accepts the authority to become the new rewarder.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_authority(ctx: Context<AcceptAuthority>) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;
        let next_authority = rewarder.pending_authority;
        assert_keys!(ctx.accounts.authority, next_authority, "pending authority");
        rewarder.authority = next_authority;
        rewarder.pending_authority = Pubkey::default();
        Ok(())
    }

    /// Sets the amount of reward tokens distributed to all [Quarry]s per day.
    #[access_control(ctx.accounts.validate())]
    pub fn set_annual_rewards(ctx: Context<SetAnnualRewards>, new_rate: u64) -> ProgramResult {
        require!(
            new_rate <= MAX_ANNUAL_REWARDS_RATE,
            MaxAnnualRewardsRateExceeded
        );
        let rewarder = &mut ctx.accounts.auth.rewarder;
        let previous_rate = rewarder.annual_rewards_rate;
        rewarder.annual_rewards_rate = new_rate;

        emit!(RewarderAnnualRewardsUpdateEvent {
            previous_rate,
            new_rate,
            timestamp: ctx.accounts.clock.unix_timestamp,
        });

        Ok(())
    }

    /// --------------------------------
    /// Quarry functions
    /// --------------------------------

    /// Creates a new [Quarry].
    /// This may only be called by the [Rewarder]::authority.
    #[access_control(ctx.accounts.validate())]
    pub fn create_quarry(ctx: Context<CreateQuarry>, bump: u8) -> ProgramResult {
        let rewarder = &mut ctx.accounts.auth.rewarder;
        // Update rewarder's quarry stats
        rewarder.num_quarries = unwrap_int!(rewarder.num_quarries.checked_add(1));

        let quarry = &mut ctx.accounts.quarry;
        quarry.bump = bump;

        // Set quarry params
        quarry.famine_ts = i64::MAX;
        quarry.rewarder_key = *rewarder.to_account_info().key;
        quarry.annual_rewards_rate = 0;
        quarry.rewards_share = 0;
        quarry.token_mint_decimals = ctx.accounts.token_mint.decimals;
        quarry.token_mint_key = *ctx.accounts.token_mint.to_account_info().key;

        emit!(QuarryCreateEvent {
            token_mint: quarry.token_mint_key,
            timestamp: ctx.accounts.clock.unix_timestamp,
        });

        Ok(())
    }

    /// Sets the rewards share of a quarry.
    #[access_control(ctx.accounts.validate())]
    pub fn set_rewards_share(ctx: Context<SetRewardsShare>, new_share: u64) -> ProgramResult {
        let rewarder = &mut ctx.accounts.auth.rewarder;
        let quarry = &mut ctx.accounts.quarry;
        rewarder.total_rewards_shares = unwrap_int!(rewarder
            .total_rewards_shares
            .checked_add(new_share)
            .and_then(|v| v.checked_sub(quarry.rewards_share)));

        quarry.last_update_ts = cmp::min(ctx.accounts.clock.unix_timestamp, quarry.famine_ts);
        quarry.annual_rewards_rate = rewarder.compute_quarry_annual_rewards_rate(new_share)?;
        quarry.rewards_share = new_share;

        emit!(QuarryRewardsUpdateEvent {
            token_mint: quarry.token_mint_key,
            annual_rewards_rate: quarry.annual_rewards_rate,
            rewards_share: quarry.rewards_share,
            timestamp: ctx.accounts.clock.unix_timestamp,
        });

        Ok(())
    }

    /// Sets the famine, which stops rewards.
    #[access_control(ctx.accounts.validate())]
    pub fn set_famine(ctx: Context<SetFamine>, famine_ts: i64) -> ProgramResult {
        let quarry = &mut ctx.accounts.quarry;
        quarry.famine_ts = famine_ts;

        Ok(())
    }

    /// Synchronizes quarry rewards with the rewarder.
    /// Anyone can call this.
    #[access_control(ctx.accounts.validate())]
    pub fn update_quarry_rewards(ctx: Context<UpdateQuarryRewards>) -> ProgramResult {
        let current_ts = ctx.accounts.clock.unix_timestamp;
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
    #[access_control(ctx.accounts.validate())]
    pub fn create_miner(ctx: Context<CreateMiner>, bump: u8) -> ProgramResult {
        let miner = &mut ctx.accounts.miner;
        miner.authority = ctx.accounts.authority.key();
        miner.bump = bump;
        miner.quarry_key = ctx.accounts.quarry.key();
        miner.token_vault_key = ctx.accounts.miner_vault.key();
        miner.rewards_earned = 0;
        miner.rewards_per_token_paid = 0;
        miner.balance = 0;

        emit!(MinerCreateEvent {
            authority: miner.authority,
            quarry: miner.quarry_key,
            miner: miner.key(),
        });

        Ok(())
    }

    /// Claims rewards for the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> ProgramResult {
        let miner = &mut ctx.accounts.stake.miner;

        let clock = &ctx.accounts.stake.clock;
        let quarry = &mut ctx.accounts.stake.quarry;
        quarry.update_rewards_and_miner(
            miner,
            &ctx.accounts.stake.rewarder,
            clock.unix_timestamp,
        )?;

        let amount_claimable = miner.rewards_earned;
        if amount_claimable == 0 {
            // 0 claimable -- skip all logic
            return Ok(());
        }
        require!(
            amount_claimable <= ctx.accounts.minter.allowance,
            InsufficientAllowance
        );

        // Calculate rewards
        let max_claim_fee_kbps = ctx.accounts.stake.rewarder.max_claim_fee_kbps;
        require!(max_claim_fee_kbps < 10_000 * 1_000, InvalidMaxClaimFee);
        let max_claim_fee = unwrap_int!(unwrap_int!((amount_claimable as u128)
            .checked_mul(max_claim_fee_kbps.into())
            .and_then(|f| f.checked_div((10_000 * 1_000) as u128)))
        .to_u64());

        let amount_claimable_minus_fees = unwrap_int!(amount_claimable.checked_sub(max_claim_fee));

        // Create the signer seeds.
        let seeds = gen_rewarder_signer_seeds!(ctx.accounts.stake.rewarder);
        let signer_seeds = &[&seeds[..]];

        // Mint the claimed tokens.
        quarry_mint_wrapper::cpi::perform_mint(
            CpiContext::new_with_signer(
                ctx.accounts.mint_wrapper_program.clone(),
                quarry_mint_wrapper::PerformMint {
                    mint_wrapper: ctx.accounts.mint_wrapper.clone().into(),
                    minter_authority: ctx.accounts.stake.rewarder.to_account_info(),
                    token_mint: ctx.accounts.rewards_token_mint.clone(),
                    destination: ctx.accounts.rewards_token_account.clone(),
                    minter: ProgramAccount::<quarry_mint_wrapper::Minter>::try_from(
                        &ctx.accounts.minter.to_account_info(),
                    )?,
                    token_program: ctx.accounts.stake.token_program.clone(),
                },
                signer_seeds,
            ),
            amount_claimable_minus_fees,
        )?;

        // Mint the fees.
        quarry_mint_wrapper::cpi::perform_mint(
            CpiContext::new_with_signer(
                ctx.accounts.mint_wrapper_program.clone(),
                quarry_mint_wrapper::PerformMint {
                    mint_wrapper: ctx.accounts.mint_wrapper.clone().into(),
                    minter_authority: ctx.accounts.stake.rewarder.to_account_info(),
                    token_mint: ctx.accounts.rewards_token_mint.clone(),
                    destination: ctx.accounts.claim_fee_token_account.clone(),
                    minter: ProgramAccount::<quarry_mint_wrapper::Minter>::try_from(
                        &ctx.accounts.minter.to_account_info(),
                    )?,
                    token_program: ctx.accounts.stake.token_program.clone(),
                },
                signer_seeds,
            ),
            max_claim_fee,
        )?;
        miner.rewards_earned = 0;

        emit!(ClaimEvent {
            authority: ctx.accounts.stake.authority.key(),
            staked_token: ctx.accounts.stake.token_account.mint,
            timestamp: clock.unix_timestamp,
            rewards_token: ctx.accounts.rewards_token_mint.key(),
            amount: amount_claimable_minus_fees,
            fees: max_claim_fee,
        });

        Ok(())
    }

    /// Stakes tokens into the [Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn stake_tokens(ctx: Context<UserStake>, amount: u64) -> ProgramResult {
        if amount == 0 {
            // noop
            return Ok(());
        }

        let quarry = &mut ctx.accounts.quarry;
        let clock = &ctx.accounts.clock;
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
            authority: ctx.accounts.authority.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
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
    pub fn withdraw_tokens(ctx: Context<UserStake>, amount: u64) -> ProgramResult {
        if amount == 0 {
            // noop
            return Ok(());
        }
        require!(
            amount <= ctx.accounts.miner_vault.amount,
            InsufficientBalance
        );

        let clock = &ctx.accounts.clock;
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
            ctx.accounts.miner.quarry_key.as_ref(),
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
            ctx.accounts.token_program.clone(),
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

    /// --------------------------------
    /// Protocol Functions
    /// --------------------------------

    /// Extracts fees to the Quarry DAO.
    /// This can be called by anyone.
    #[access_control(ctx.accounts.validate())]
    pub fn extract_fees(ctx: Context<ExtractFees>) -> ProgramResult {
        let seeds = gen_rewarder_signer_seeds!(ctx.accounts.rewarder);
        let signer_seeds = &[&seeds[..]];

        // Transfer the tokens to the DAO address.
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.clone(),
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
/// PDA Structs
/// --------------------------------

/// Controls token rewards distribution to all [Quarry]s.
/// The [Rewarder] is also the [Minter] registered to the [MintWrapper].
#[account]
#[derive(Default, Debug)]
pub struct Rewarder {
    /// Random pubkey used for generating the program address.
    pub base: Pubkey,
    /// Bump seed for program address.
    pub bump: u8,

    /// Authority who controls the rewarder
    pub authority: Pubkey,
    /// Pending authority which must accept the authority
    pub pending_authority: Pubkey,

    /// Number of [Quarry]s the [Rewarder] manages.
    /// If more than this many [Quarry]s are desired, one can create
    /// a second rewarder.
    pub num_quarries: u16,
    /// Amount of reward tokens distributed per day
    pub annual_rewards_rate: u64,
    /// Total amount of rewards shares allocated to [Quarry]s
    pub total_rewards_shares: u64,
    /// Mint wrapper.
    pub mint_wrapper: Pubkey,
    /// Mint of the rewards token for this [Rewarder].
    pub rewards_token_mint: Pubkey,

    /// Claim fees are placed in this account.
    pub claim_fee_token_account: Pubkey,
    /// Maximum amount of tokens to send to the Quarry DAO on each claim,
    /// in terms of thousands of BPS.
    /// This is stored on the [Rewarder] to ensure that the fee will
    /// not exceed this in the future.
    pub max_claim_fee_kbps: u64,

    /// Authority allowed to pause a [Rewarder].
    pub pause_authority: Pubkey,
    /// If true, all instructions on the [Rewarder] are paused other than [quarry_mine::unpause].
    pub is_paused: bool,
}

/// A pool which distributes tokens to its [Miner]s.
#[account]
#[derive(Copy, Default)]
pub struct Quarry {
    /// Rewarder who owns this quarry
    pub rewarder_key: Pubkey,
    /// LP token this quarry is designated to
    pub token_mint_key: Pubkey,
    /// Bump.
    pub bump: u8,

    /// Decimals on the token mint
    pub token_mint_decimals: u8,
    /// Timestamp when quarry rewards cease
    pub famine_ts: i64,
    /// Timestamp of last checkpoint
    pub last_update_ts: i64,
    /// Rewards per token stored in the quarry
    pub rewards_per_token_stored: u128,
    /// Amount of rewards distributed to the quarry per year.
    pub annual_rewards_rate: u64,
    /// Rewards shared allocated to this quarry
    pub rewards_share: u64,

    /// Total number of tokens deposited into the quarry.
    pub total_tokens_deposited: u64,
}

/// An account that has staked tokens into a [Quarry].
#[account]
#[derive(Default)]
pub struct Miner {
    /// Key of the [Quarry] this [Miner] works on.
    pub quarry_key: Pubkey,
    /// Authority who manages this [Miner].
    /// All withdrawals of tokens must accrue to [TokenAccount]s owned by this account.
    pub authority: Pubkey,

    /// Bump.
    pub bump: u8,

    /// [TokenAccount] to hold the [Miner]'s staked LP tokens.
    pub token_vault_key: Pubkey,

    /// Stores the amount of tokens that the [Miner] may claim.
    /// Whenever the [Miner] claims tokens, this is reset to 0.
    pub rewards_earned: u64,

    /// A checkpoint of the [Quarry]'s reward tokens paid per staked token.
    ///
    /// When the [Miner] is initialized, this number starts at 0.
    /// On the first [quarry_mine::stake_tokens], the [Quarry]#update_rewards_and_miner
    /// method is called, which updates this checkpoint to the current quarry value.
    ///
    /// On a [quarry_mine::claim_rewards], the difference in checkpoints is used to calculate
    /// the amount of tokens owed.
    pub rewards_per_token_paid: u128,

    /// Number of tokens the [Miner] holds.
    pub balance: u64,
}

/// --------------------------------
/// Context Structs
/// --------------------------------

/* Rewarder contexts */

/// Accounts for [quarry_mine::new_rewarder].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct NewRewarder<'info> {
    /// Base. Arbitrary key.
    #[account(signer)]
    pub base: AccountInfo<'info>,

    /// [Rewarder] of mines.
    #[account(
        init,
        seeds = [
            b"Rewarder".as_ref(),
            base.key().to_bytes().as_ref(),
            &[bump],
        ],
        payer = payer
    )]
    pub rewarder: ProgramAccount<'info, Rewarder>,

    /// Initial authority of the rewarder.
    pub authority: AccountInfo<'info>,

    /// Payer of the rewarder initialization.
    #[account(signer)]
    pub payer: AccountInfo<'info>,

    /// System program.
    pub system_program: AccountInfo<'info>,

    /// Clock.
    pub clock: Sysvar<'info, Clock>,

    /// Mint wrapper.
    pub mint_wrapper: CpiAccount<'info, quarry_mint_wrapper::MintWrapper>,

    /// Rewards token mint.
    pub rewards_token_mint: CpiAccount<'info, Mint>,

    /// Token account in which the rewards token fees are collected.
    pub claim_fee_token_account: CpiAccount<'info, TokenAccount>,
}

/// Accounts for [quarry_mine::set_pause_authority].
#[derive(Accounts)]
pub struct SetPauseAuthority<'info> {
    /// [Rewarder].
    pub auth: MutableRewarderWithAuthority<'info>,

    /// The pause authority.
    pub pause_authority: AccountInfo<'info>,
}

/// Accounts for [quarry_mine::transfer_authority].
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    /// Authority of the rewarder.
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: ProgramAccount<'info, Rewarder>,
}

/// Accounts for [quarry_mine::accept_authority].
#[derive(Accounts)]
pub struct AcceptAuthority<'info> {
    /// Authority of the next rewarder.
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: ProgramAccount<'info, Rewarder>,
}

/// Mutable [Rewarder] that requires the authority to be a signer.
#[derive(Accounts)]
pub struct MutableRewarderWithAuthority<'info> {
    /// Authority of the rewarder.
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: ProgramAccount<'info, Rewarder>,
}

/// Read-only [Rewarder] that requires the authority to be a signer.
#[derive(Accounts)]
pub struct ReadOnlyRewarderWithAuthority<'info> {
    /// Authority of the rewarder.
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// Rewarder of the farm.
    pub rewarder: ProgramAccount<'info, Rewarder>,
}

/// Accounts for [quarry_mine::set_annual_rewards].
#[derive(Accounts)]
pub struct SetAnnualRewards<'info> {
    /// [Rewarder],
    pub auth: MutableRewarderWithAuthority<'info>,

    /// [Clock].
    pub clock: Sysvar<'info, Clock>,
}

/* Quarry contexts */

/// Accounts for [quarry_mine::create_quarry].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateQuarry<'info> {
    /// [Quarry].
    #[account(
        init,
        seeds = [
            b"Quarry".as_ref(),
            auth.rewarder.key().to_bytes().as_ref(),
            token_mint.key().to_bytes().as_ref(),
            &[bump],
        ],
        payer = payer
    )]
    pub quarry: ProgramAccount<'info, Quarry>,

    /// [Rewarder] authority.
    pub auth: MutableRewarderWithAuthority<'info>,

    /// [Mint] of the token to create a [Quarry] for.
    pub token_mint: CpiAccount<'info, Mint>,

    /// Payer of [Quarry] creation.
    pub payer: AccountInfo<'info>,

    /// [Clock].
    pub clock: Sysvar<'info, Clock>,

    /// System program.
    pub system_program: AccountInfo<'info>,
}

/// Accounts for [quarry_mine::set_famine].
#[derive(Accounts)]
pub struct SetFamine<'info> {
    /// [Rewarder] of the [Quarry].
    pub auth: ReadOnlyRewarderWithAuthority<'info>,

    /// [Quarry] updated.
    #[account(mut)]
    pub quarry: ProgramAccount<'info, Quarry>,
}

/// Accounts for [quarry_mine::set_rewards_share].
#[derive(Accounts)]
pub struct SetRewardsShare<'info> {
    /// [Rewarder] of the [Quarry].
    pub auth: MutableRewarderWithAuthority<'info>,

    /// [Quarry] updated.
    #[account(mut)]
    pub quarry: ProgramAccount<'info, Quarry>,

    /// [Clock].
    pub clock: Sysvar<'info, Clock>,
}

/// Accounts for [quarry_mine::update_quarry_rewards].
#[derive(Accounts)]
pub struct UpdateQuarryRewards<'info> {
    /// [Quarry].
    #[account(mut)]
    pub quarry: ProgramAccount<'info, Quarry>,

    /// [Rewarder].
    pub rewarder: ProgramAccount<'info, Rewarder>,

    /// [Clock].
    pub clock: Sysvar<'info, Clock>,
}

/* Miner contexts */

/// Accounts for [quarry_mine::create_miner].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateMiner<'info> {
    /// Authority of the [Miner].
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// [Miner] to be created.
    #[account(
        init,
        seeds = [
            b"Miner".as_ref(),
            quarry.key().to_bytes().as_ref(),
            authority.key().to_bytes().as_ref(),
            &[bump],
        ],
        payer = payer
    )]
    pub miner: ProgramAccount<'info, Miner>,

    /// [Quarry] to create a [Miner] for.
    pub quarry: ProgramAccount<'info, Quarry>,

    /// [Rewarder].
    pub rewarder: ProgramAccount<'info, Rewarder>,

    /// System program.
    pub system_program: AccountInfo<'info>,

    /// Payer of [Miner] creation.
    pub payer: AccountInfo<'info>,

    /// [Mint] of the token to create a [Quarry] for.
    pub token_mint: CpiAccount<'info, Mint>,

    /// [TokenAccount] holding the token [Mint].
    pub miner_vault: CpiAccount<'info, TokenAccount>,

    /// SPL Token program.
    pub token_program: AccountInfo<'info>,
}

/// ClaimRewards accounts
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    /// Mint wrapper.
    pub mint_wrapper: CpiAccount<'info, quarry_mint_wrapper::MintWrapper>,
    /// Mint wrapper program.
    pub mint_wrapper_program: AccountInfo<'info>,
    /// [quarry_mint_wrapper::Minter] information.
    #[account(mut)]
    pub minter: CpiAccount<'info, quarry_mint_wrapper::Minter>,

    /// Mint of the rewards token.
    #[account(mut)]
    pub rewards_token_mint: CpiAccount<'info, Mint>,

    /// Account to claim rewards for.
    #[account(mut)]
    pub rewards_token_account: CpiAccount<'info, TokenAccount>,

    /// Account to send claim fees to.
    #[account(mut)]
    pub claim_fee_token_account: CpiAccount<'info, TokenAccount>,

    /// User's stake.
    pub stake: UserStake<'info>,
}

/// Staking accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
#[derive(Accounts, Clone)]
pub struct UserStake<'info> {
    /// Miner authority (i.e. the user).
    #[account(signer)]
    pub authority: AccountInfo<'info>,

    /// Miner.
    #[account(mut)]
    pub miner: ProgramAccount<'info, Miner>,

    /// Quarry to claim from.
    #[account(mut)]
    pub quarry: ProgramAccount<'info, Quarry>,

    /// Vault of the miner.
    #[account(mut)]
    pub miner_vault: CpiAccount<'info, TokenAccount>,

    /// User's staked token account
    #[account(mut)]
    pub token_account: CpiAccount<'info, TokenAccount>,

    /// Token program
    pub token_program: AccountInfo<'info>,

    /// Rewarder
    pub rewarder: ProgramAccount<'info, Rewarder>,

    /// Clock
    pub clock: Sysvar<'info, Clock>,
}

/// Accounts for [quarry_mine::extract_fees].
#[derive(Accounts)]
pub struct ExtractFees<'info> {
    /// Rewarder to extract fees from.
    pub rewarder: ProgramAccount<'info, Rewarder>,

    /// [TokenAccount] which receives claim fees.
    #[account(mut)]
    pub claim_fee_token_account: CpiAccount<'info, TokenAccount>,

    /// [TokenAccount] owned by the [addresses::FEE_TO_ADDRESS].
    /// Holds DAO claim fees.
    #[account(mut)]
    pub fee_to_token_account: CpiAccount<'info, TokenAccount>,

    /// Token program
    pub token_program: AccountInfo<'info>,
}

/// Accounts for [quarry_mine::pause] and [quarry_mine::unpause].
#[derive(Accounts)]
pub struct MutableRewarderWithPauseAuthority<'info> {
    /// Pause authority of the rewarder.
    #[account(signer)]
    pub pause_authority: AccountInfo<'info>,

    /// Rewarder of the farm.
    #[account(mut)]
    pub rewarder: ProgramAccount<'info, Rewarder>,
}

/// --------------------------------
/// Events
/// --------------------------------

/// Emitted when a new rewarder is created
#[event]
pub struct NewRewarderEvent {
    /// Authority of the rewarder
    #[index]
    pub authority: Pubkey,
    /// When the event occurred.
    pub timestamp: i64,
}

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

/// Triggered when the daily rewards rate is updated.
#[event]
pub struct RewarderAnnualRewardsUpdateEvent {
    /// Previous rate of rewards.
    pub previous_rate: u64,
    /// New rate of rewards.
    pub new_rate: u64,
    /// When the event took place.
    pub timestamp: i64,
}

/// Triggered when a new miner is created.
#[event]
pub struct MinerCreateEvent {
    /// Authority of the miner.
    #[index]
    pub authority: Pubkey,
    /// Quarry the miner was created on.
    #[index]
    pub quarry: Pubkey,
    /// The [Miner].
    pub miner: Pubkey,
}

/// Triggered when a new quarry is created.
#[event]
pub struct QuarryCreateEvent {
    /// [Mint] of the [Quarry] token.
    pub token_mint: Pubkey,
    /// When the event took place.
    pub timestamp: i64,
}

/// Triggered when a quarry's reward share is updated.
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
#[error]
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
}
