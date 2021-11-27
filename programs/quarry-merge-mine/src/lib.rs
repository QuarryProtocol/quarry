//! Holds tokens to allow one depositor to mine multiple quarries at the same time.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

#[macro_use]
mod macros;

mod account_validators;
mod processor;

pub(crate) mod account_conversions;
pub(crate) mod mm_cpi;

pub mod events;
pub mod state;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use vipers::validate::Validate;

use state::*;

declare_id!("QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto");

#[deny(clippy::integer_arithmetic, clippy::float_arithmetic)]
#[program]
/// Quarry merge mining program.
pub mod quarry_merge_mine {
    use super::*;

    /// Creates a new [MergePool].
    /// Anyone can call this.
    #[access_control(ctx.accounts.validate())]
    pub fn new_pool(ctx: Context<NewPool>, bump: u8, _mint_bump: u8) -> ProgramResult {
        processor::init::new_pool(ctx, bump)
    }

    /// Creates a new [MergeMiner].
    /// Anyone can call this.
    #[access_control(ctx.accounts.validate())]
    pub fn init_merge_miner(ctx: Context<InitMergeMiner>, bump: u8) -> ProgramResult {
        processor::init::init_merge_miner(ctx, bump)
    }

    /// Initializes a [quarry_mine::Miner] owned by the [MergeMiner].
    #[access_control(ctx.accounts.validate())]
    pub fn init_miner(ctx: Context<InitMiner>, bump: u8) -> ProgramResult {
        processor::init::init_miner(ctx, bump)
    }

    // --------------------------------
    // Deposit
    // --------------------------------

    /// Deposits tokens into the [MergeMiner].
    /// Before calling this, the owner should call the [anchor_spl::token::transfer] instruction
    /// to transfer to the [MergeMiner]'s primary token ATA.
    #[access_control(ctx.accounts.validate())]
    pub fn stake_primary_miner(ctx: Context<QuarryStakePrimary>) -> ProgramResult {
        processor::deposit::stake_primary_miner(ctx)
    }

    /// Stakes all possible replica tokens into a [quarry_mine::Quarry].
    /// Before calling this, the owner should call [stake_primary_miner] with the tokens
    /// they would like to stake.
    #[access_control(ctx.accounts.validate())]
    pub fn stake_replica_miner(ctx: Context<QuarryStakeReplica>) -> ProgramResult {
        processor::deposit::stake_replica_miner(ctx)
    }

    // --------------------------------
    // Withdraw
    // --------------------------------

    /// Withdraws tokens from the [MergeMiner].
    #[access_control(ctx.accounts.validate())]
    pub fn unstake_primary_miner(ctx: Context<QuarryStakePrimary>, amount: u64) -> ProgramResult {
        processor::withdraw::unstake_primary_miner(ctx, amount)
    }

    /// Unstakes all of a [MergeMiner]'s replica tokens for a [quarry_mine::Miner].
    #[access_control(ctx.accounts.validate())]
    pub fn unstake_all_replica_miner(ctx: Context<QuarryStakeReplica>) -> ProgramResult {
        processor::withdraw::unstake_all_replica_miner(ctx)
    }

    /// Withdraws tokens from the [MergeMiner].
    #[access_control(ctx.accounts.validate())]
    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>) -> ProgramResult {
        processor::withdraw::withdraw_tokens(ctx)
    }

    // --------------------------------
    // Claim
    // --------------------------------

    /// Claims [quarry_mine] rewards on behalf of the [MergeMiner].
    #[access_control(ctx.accounts.validate())]
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> ProgramResult {
        processor::claim::claim_rewards(ctx)
    }
}

// --------------------------------
// Instruction account structs
// --------------------------------

/// [quarry_merge_mine::new_pool] accounts
#[derive(Accounts)]
#[instruction(bump: u8, mint_bump: u8)]
pub struct NewPool<'info> {
    /// [MergePool].
    #[account(
        init,
        seeds = [
          b"MergePool",
          primary_mint.key().to_bytes().as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub pool: Account<'info, MergePool>,

    /// [Mint] of the primary (underlying) token.
    pub primary_mint: Account<'info, Mint>,

    /// [Mint] of the replica token.
    #[account(
        init,
        seeds = [
            b"ReplicaMint",
            pool.key().to_bytes().as_ref()
        ],
        mint::decimals = primary_mint.decimals,
        mint::authority = pool,
        bump = mint_bump,
        payer = payer,
        space = Mint::LEN
    )]
    pub replica_mint: Account<'info, Mint>,

    /// Payer of the created [MergePool].
    #[account(mut)]
    pub payer: Signer<'info>,

    /// [Token] program.
    pub token_program: Program<'info, Token>,

    /// [System] program.
    pub system_program: Program<'info, System>,

    /// [Rent] sysvar.
    pub rent: Sysvar<'info, Rent>,
}

/// [quarry_merge_mine::init_merge_miner] accounts
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitMergeMiner<'info> {
    /// [MergePool] of the underlying LP token.
    pub pool: Account<'info, MergePool>,

    /// Owner of the [MergeMiner].
    pub owner: UncheckedAccount<'info>,

    /// [MergeMiner].
    #[account(
        init,
        seeds = [
          b"MergeMiner",
          pool.key().to_bytes().as_ref(),
          owner.key().to_bytes().as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub mm: Account<'info, MergeMiner>,

    /// Payer of the created [MergeMiner].
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

/// [quarry_merge_mine::init_miner] accounts
#[derive(Accounts)]
pub struct InitMiner<'info> {
    /// The [MergePool].
    pub pool: Account<'info, MergePool>,

    /// The [MergeMiner], aka the authority of the [quarry_mine::Miner].
    pub mm: Account<'info, MergeMiner>,

    /// [quarry_mine::Miner] to be created.
    #[account(mut)]
    pub miner: UncheckedAccount<'info>,

    /// [quarry_mine::Quarry] to create a [quarry_mine::Miner] for.
    #[account(mut)]
    pub quarry: Box<Account<'info, quarry_mine::Quarry>>,

    /// [quarry_mine::Rewarder].
    pub rewarder: Box<Account<'info, quarry_mine::Rewarder>>,

    /// [Mint] of the Quarry token.
    pub token_mint: Box<Account<'info, Mint>>,

    /// [TokenAccount] holding the token [Mint].
    pub miner_vault: Account<'info, TokenAccount>,

    /// Payer of [quarry_mine::Miner] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The program at [quarry_mine::ID].
    pub mine_program: Program<'info, quarry_mine::program::QuarryMine>,

    /// System program.
    pub system_program: Program<'info, System>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// [quarry_merge_mine::withdraw_tokens] accounts
#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    /// Owner of the [MergeMiner].
    pub owner: Signer<'info>,
    /// The [MergePool] to withdraw from.
    pub pool: Account<'info, MergePool>,
    /// The [MergeMiner] to withdraw from.
    #[account(mut)]
    pub mm: Account<'info, MergeMiner>,

    /// The [Mint] being withdrawn from the [MergeMiner].
    pub withdraw_mint: Account<'info, Mint>,
    /// A [TokenAccount] owned by the [MergeMiner] to withdraw from.
    /// Must be the [MergePool::primary_mint] or the [MergePool::replica_mint].
    #[account(mut)]
    pub mm_token_account: Account<'info, TokenAccount>,
    /// Account to send tokens to.
    #[account(mut)]
    pub token_destination: Account<'info, TokenAccount>,

    /// The token program
    pub token_program: Program<'info, Token>,
}

/// [quarry_merge_mine::claim_rewards] accounts
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    /// Mint wrapper.
    #[account(mut)]
    pub mint_wrapper: Box<Account<'info, quarry_mint_wrapper::MintWrapper>>,
    /// Mint wrapper program.
    pub mint_wrapper_program: Program<'info, quarry_mint_wrapper::program::QuarryMintWrapper>,
    /// [quarry_mint_wrapper::Minter].
    #[account(mut)]
    pub minter: Box<Account<'info, quarry_mint_wrapper::Minter>>,

    /// [Mint] of the [quarry_mine] rewards token.
    #[account(mut)]
    pub rewards_token_mint: Box<Account<'info, Mint>>,

    /// Account to claim rewards for.
    #[account(mut)]
    pub rewards_token_account: Box<Account<'info, TokenAccount>>,

    /// Account to send claim fees to.
    #[account(mut)]
    pub claim_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// Arbitrary account holding the [Mint] of the quarry staked token.
    /// Passed to [quarry_mine] but unused.
    #[account(mut)]
    pub stake_token_account: Box<Account<'info, TokenAccount>>,

    /// User's stake.
    pub stake: QuarryStake<'info>,
}

/// [quarry_merge_mine::stake_primary_miner] accounts
#[derive(Accounts)]
pub struct QuarryStakePrimary<'info> {
    /// The [MergeMiner::owner].
    pub mm_owner: Signer<'info>,

    /// The [TokenAccount] holding the [MergeMiner]'s primary tokens.
    #[account(mut)]
    pub mm_primary_token_account: Account<'info, TokenAccount>,

    /// Staking accounts for the [quarry_mine::Quarry].
    pub stake: QuarryStake<'info>,
}

/// [quarry_merge_mine::stake_replica_miner] accounts
#[derive(Accounts)]
pub struct QuarryStakeReplica<'info> {
    /// The [MergeMiner::owner].
    pub mm_owner: Signer<'info>,

    /// [Mint] of a token that can be staked into a farming program.
    /// This token should not be distributed to users, as it can depeg and can cause minters to lose their funds.
    /// The [MergePool] must be the `mint_authority` and the `freeze_authority`.
    #[account(mut)]
    pub replica_mint: Account<'info, Mint>,

    /// The [TokenAccount] holding the [MergeMiner]'s minted pool tokens.
    #[account(mut)]
    pub replica_mint_token_account: Account<'info, TokenAccount>,

    /// Staking accounts for the [quarry_mine::Quarry].
    pub stake: QuarryStake<'info>,
}

// --------------------------------
// Context Structs
// --------------------------------

/// Staking accounts for a [quarry_mine::Quarry].
#[derive(Accounts)]
pub struct QuarryStake<'info> {
    /// The [MergePool].
    #[account(mut)]
    pub pool: Account<'info, MergePool>,

    /// The [MergeMiner] (also the [quarry_mine::Miner] authority).
    #[account(mut)]
    pub mm: Account<'info, MergeMiner>,

    /// The [quarry_mine::Rewarder] to stake into.
    pub rewarder: Box<Account<'info, quarry_mine::Rewarder>>,

    /// The [quarry_mine::Quarry] to claim from.
    #[account(mut)]
    pub quarry: Box<Account<'info, quarry_mine::Quarry>>,

    /// The [quarry_mine::Miner].
    #[account(mut)]
    pub miner: Box<Account<'info, quarry_mine::Miner>>,

    /// The [TokenAccount] of the [quarry_mine::Miner] that holds the staked tokens.
    #[account(mut)]
    pub miner_vault: Account<'info, TokenAccount>,

    /// [anchor_spl::token] program.
    pub token_program: Program<'info, Token>,

    /// [quarry_mine] program.
    pub mine_program: Program<'info, quarry_mine::program::QuarryMine>,

    /// Unused variable used as a filler for deprecated accounts. Handled by [quarry_mine].
    /// One should pass in a randomly generated Keypair for this account.
    #[account(mut)]
    pub unused_account: UncheckedAccount<'info>,
}

/// Error Codes
#[error]
pub enum ErrorCode {
    #[msg("Unauthorized.")]
    Unauthorized,
    #[msg("Insufficient balance.")]
    InsufficientBalance,
    #[msg("Invalid miner for the given quarry.")]
    InvalidMiner,
    #[msg("Cannot withdraw a replica mint.")]
    CannotWithdrawReplicaMint,
    #[msg("User must first withdraw from all replica quarries.")]
    OutstandingReplicaTokens,
}
