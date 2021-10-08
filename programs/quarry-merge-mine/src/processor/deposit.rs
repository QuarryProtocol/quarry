//! Deposit-related instructions.

use crate::{events::*, QuarryStakePrimary, QuarryStakeReplica};
use anchor_lang::prelude::*;
use vipers::*;

/// Deposits tokens into the [MergeMiner].
/// Before calling this, the owner should call the [token::transfer] instruction
/// to transfer to the [MergeMiner]'s [MergeMiner]::primary_token_account.
pub fn stake_primary_miner(ctx: Context<QuarryStakePrimary>) -> ProgramResult {
    let mm = &ctx.accounts.stake.mm;
    let amount = mm.stake_max_primary_miner(ctx.accounts)?;

    // Update [MergeMiner]/[MergePool]
    let mm = &mut ctx.accounts.stake.mm;
    mm.primary_balance = unwrap_int!(mm.primary_balance.checked_add(amount));
    let pool = &mut ctx.accounts.stake.pool;
    pool.total_primary_balance = unwrap_int!(pool.total_primary_balance.checked_add(amount));

    ctx.accounts.stake.miner.reload()?;
    invariant!(
        mm.primary_balance == ctx.accounts.stake.miner.balance,
        "after ix, mm balance must be miner balance"
    );

    emit!(StakePrimaryEvent {
        pool: pool.key(),
        mm: mm.key(),
        miner: ctx.accounts.stake.miner.key(),
        owner: mm.owner.key(),
        amount,
    });

    Ok(())
}

/// Stakes all possible replica tokens into a [quarry_mine::Quarry].
/// Before calling this, the owner should call the [anchor_spl::token::transfer] instruction
/// to transfer to the [MergeMiner]'s primary token ATA.
pub fn stake_replica_miner(ctx: Context<QuarryStakeReplica>) -> ProgramResult {
    // ! IMPORTANT NOTE !
    // The replica mint issued to this pool could greatly exceed that of the balance.
    // However, withdrawals are only possible if all of the replica miners are unstaked,
    // since the total outstanding replica tokens issued to a given [MergeMiner] must be zero.

    // This only works because there can only be one Quarry per mint per rewarder.
    // If one desires multiple Quarries per mint per rewarder, one should incentivize a wrapper token.

    // pre-instruction checks
    let pre_replica_mint_supply = ctx.accounts.replica_mint.supply;
    invariant!(
        ctx.accounts.stake.miner.balance == ctx.accounts.stake.miner_vault.amount,
        "replica miner balance should equal miner_vault balance"
    );

    let mm = &ctx.accounts.stake.mm;
    let stake_amount = mm.stake_max_replica_miner(ctx.accounts)?;

    // Update replica balance
    let mm = &mut ctx.accounts.stake.mm;
    mm.replica_balance = unwrap_int!(mm.replica_balance.checked_add(stake_amount));
    let pool = &mut ctx.accounts.stake.pool;
    pool.total_replica_balance = unwrap_int!(pool.total_replica_balance.checked_add(stake_amount));

    emit!(StakeReplicaEvent {
        pool: pool.key(),
        mm: mm.key(),
        miner: ctx.accounts.stake.miner.key(),
        owner: mm.owner.key(),
        amount: stake_amount,
    });

    // post-instruction checks
    post_stake_replica_miner(ctx, pre_replica_mint_supply, stake_amount)?;

    Ok(())
}

/// Checks run after [crate::quarry_merge_mine::stake_replica_miner].
fn post_stake_replica_miner(
    ctx: Context<QuarryStakeReplica>,
    pre_replica_mint_supply: u64,
    stake_amount: u64,
) -> ProgramResult {
    ctx.accounts.stake.miner.reload()?;
    ctx.accounts.stake.miner_vault.reload()?;
    ctx.accounts.replica_mint.reload()?;
    ctx.accounts.replica_mint_token_account.reload()?;
    invariant!(
        unwrap_int!(ctx
            .accounts
            .replica_mint
            .supply
            .checked_sub(pre_replica_mint_supply))
            == stake_amount,
        "supply increase should equal the stake amount"
    );
    invariant!(
        ctx.accounts.stake.mm.primary_balance == ctx.accounts.stake.miner.balance,
        "replica miner balance should equal primary_balance"
    );
    invariant!(
        ctx.accounts.stake.miner.balance == ctx.accounts.stake.miner_vault.amount,
        "replica miner balance should equal miner_vault balance"
    );
    invariant!(
        ctx.accounts.replica_mint_token_account.amount == 0,
        "mm replica mint balance should be zero"
    );

    Ok(())
}
