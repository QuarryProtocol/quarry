//! Struct definitions for accounts that hold state.

use anchor_lang::prelude::*;

/// A token that represents a locked other token.
///
/// To derive the address, use the following code:
/// ```
/// &[
///     b"MergePool" as &[u8],
///     &$pool.primary_mint.to_bytes(),
///     &[$pool.bump],
/// ]
/// ```
#[account]
#[derive(Copy, Debug, Default)]
pub struct MergePool {
    /// Mint of the underlying staked token, i.e. the [quarry_mine::Quarry::token_mint_key].
    pub primary_mint: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// Mint of the replica staked token, i.e. the [quarry_mine::Quarry::token_mint_key] of replicas.
    pub replica_mint: Pubkey,
    /// Number of [MergeMiner]s tracked by the [MergePool].
    pub mm_count: u64,

    /// Total number of primary tokens deposited.
    /// Used for TVL calculation.
    pub total_primary_balance: u64,
    /// Total number of replica tokens deposited.
    pub total_replica_balance: u64,

    /// Reserved for future program upgrades.
    pub reserved: [u64; 16],
}

/// Enables mining multiple [quarry_mine::Quarry]s simultaneously with only one deposit.
///
/// To derive the address, use the following code:
/// ```
/// &[
///   b"MergeMiner" as &[u8],
///   &$mm.pool.key().to_bytes(),
///   &$mm.owner.to_bytes(),
///   &[$mm.bump],
/// ]
/// ```
#[account]
#[derive(Copy, Debug, Default)]
pub struct MergeMiner {
    /// [MergePool] to mint against.
    pub pool: Pubkey,
    /// Owner of the [MergeMiner].
    pub owner: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// The index of the [MergeMiner] within the [MergePool].
    pub index: u64,

    /// Amount of tokens staked into the primary quarry.
    pub primary_balance: u64,
    /// Amount of replica tokens that have been issued to this [MergeMiner].
    /// Primary tokens may only be withdrawn if [MergeMiner::primary_balance] == 0 and
    /// [MergeMiner::replica_balance] == 0.
    pub replica_balance: u64,
}
