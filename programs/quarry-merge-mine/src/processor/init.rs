//! Account initialization-related instructions.

use crate::{events::*, InitMergeMiner, InitMiner, NewPool};
use anchor_lang::prelude::*;
use vipers::*;

/// Creates a new [MergePool].
/// Anyone can call this.
pub fn new_pool(ctx: Context<NewPool>, bump: u8) -> ProgramResult {
    let pool = &mut ctx.accounts.pool;
    pool.primary_mint = ctx.accounts.primary_mint.key();
    pool.bump = bump;

    pool.replica_mint = ctx.accounts.replica_mint.key();

    pool.mm_count = 0;

    pool.total_primary_balance = 0;
    pool.total_replica_balance = 0;

    emit!(NewMergePoolEvent {
        pool: pool.key(),
        primary_mint: pool.primary_mint,
    });

    Ok(())
}

/// Creates a new [MergeMiner].
/// Anyone can call this.
pub fn init_merge_miner(ctx: Context<InitMergeMiner>, bump: u8) -> ProgramResult {
    let mm = &mut ctx.accounts.mm;

    mm.pool = ctx.accounts.pool.key();
    mm.owner = ctx.accounts.owner.key();
    mm.bump = bump;

    // Track total number of pools.
    let pool = &mut ctx.accounts.pool;
    mm.index = pool.mm_count;
    pool.mm_count = unwrap_int!(pool.mm_count.checked_add(1));

    mm.primary_balance = 0;

    let primary_mint = ctx.accounts.pool.primary_mint;

    emit!(InitMergeMinerEvent {
        mm: mm.key(),
        pool: mm.pool,
        primary_mint,
        owner: mm.owner,
    });

    Ok(())
}

/// Initializes a [quarry_mine::Miner] owned by the [MergeMiner].
pub fn init_miner(ctx: Context<InitMiner>, bump: u8) -> ProgramResult {
    let mm = &ctx.accounts.mm;
    mm.init_miner(ctx.accounts, bump)?;

    emit!(InitMinerEvent {
        pool: mm.pool,
        mm: mm.key(),
        miner: ctx.accounts.miner.key()
    });

    Ok(())
}
