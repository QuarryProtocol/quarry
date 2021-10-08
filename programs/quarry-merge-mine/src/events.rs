//! Events emitted.

use crate::*;

/// Emitted when a new [MergePool] is created.
#[event]
pub struct NewMergePoolEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [Mint] staked into the [MergePool].
    pub primary_mint: Pubkey,
}

/// Emitted when a new [MergeMiner] is created.
#[event]
pub struct InitMergeMinerEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [Mint] of the primary token.
    pub primary_mint: Pubkey,
    /// Owner of the [MergeMiner].
    pub owner: Pubkey,
}

/// Emitted when a new [quarry_mine::Miner] is created.
#[event]
pub struct InitMinerEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [quarry_mine::Miner].
    pub miner: Pubkey,
}

/// Emitted when tokens are staked into the primary [quarry_mine::Miner].
#[event]
pub struct StakePrimaryEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [quarry_mine::Miner].
    pub miner: Pubkey,
    /// The owner of the [MergeMiner].
    pub owner: Pubkey,
    /// Amount staked.
    pub amount: u64,
}

/// Emitted when tokens are staked into the replica [quarry_mine::Miner].
#[event]
pub struct StakeReplicaEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [quarry_mine::Miner].
    pub miner: Pubkey,
    /// The owner of the [MergeMiner].
    pub owner: Pubkey,
    /// Amount staked.
    pub amount: u64,
}

/// Emitted when tokens are unstaked from the primary [quarry_mine::Miner].
#[event]
pub struct UnstakePrimaryEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [quarry_mine::Miner].
    pub miner: Pubkey,
    /// The owner of the [MergeMiner].
    pub owner: Pubkey,
    /// Amount unstaked.
    pub amount: u64,
}

/// Emitted when tokens are unstaked from the replica [quarry_mine::Miner].
#[event]
pub struct UnstakeReplicaEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [quarry_mine::Miner].
    pub miner: Pubkey,
    /// The owner of the [MergeMiner].
    pub owner: Pubkey,
    /// Amount unstaked.
    pub amount: u64,
}

/// Emitted when tokens are withdrawn from a [MergePool].
#[event]
pub struct WithdrawTokensEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The owner of the [MergeMiner].
    pub owner: Pubkey,
    /// The mint withdrawn.
    pub mint: Pubkey,
    /// Amount withdrawn.
    pub amount: u64,
}

/// Emitted when tokens are claimed.
#[event]
pub struct ClaimEvent {
    /// The [MergePool].
    pub pool: Pubkey,
    /// The [MergeMiner].
    pub mm: Pubkey,
    /// The [Mint] claimed.
    pub mint: Pubkey,
    /// Amount received.
    pub amount: u64,
    /// Balance before claim.
    pub initial_balance: u64,
    /// Balance after claim.
    pub end_balance: u64,
}
