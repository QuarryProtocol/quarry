//! Withdraw-related instructions.

use crate::{events::*, QuarryStakePrimary, QuarryStakeReplica, WithdrawTokens};
use anchor_lang::prelude::*;
use vipers::*;

/// Withdraws tokens from the [MergeMiner].
pub fn unstake_primary_miner(ctx: Context<QuarryStakePrimary>, amount: u64) -> ProgramResult {
    // Check to see if the [MergeMiner] is fully collateralized after the withdraw
    let mm = &ctx.accounts.stake.mm;
    require!(amount <= mm.primary_balance, InsufficientBalance);

    // There must be zero replica tokens if there is an unstaking of the primary miner.
    // This is to ensure that the user cannot mine after withdrawing their stake.
    // To perform a partial unstake, one should do a full unstake of each replica miner,
    // unstake the desired amount, then stake back into each replica miner.
    require!(mm.replica_balance == 0, OutstandingReplicaTokens);

    // Withdraw tokens to the [MergeMiner]'s account
    mm.unstake_primary_miner(ctx.accounts, amount)?;

    // Update [MergeMiner]/[MergePool]
    let mm = &mut ctx.accounts.stake.mm;
    mm.primary_balance = unwrap_int!(mm.primary_balance.checked_sub(amount));
    let pool = &mut ctx.accounts.stake.pool;
    pool.total_primary_balance = unwrap_int!(pool.total_primary_balance.checked_sub(amount));

    ctx.accounts.stake.miner.reload()?;
    invariant!(
        mm.primary_balance == ctx.accounts.stake.miner.balance,
        "after ix, mm balance must be miner balance"
    );

    emit!(UnstakePrimaryEvent {
        pool: pool.key(),
        mm: mm.key(),
        miner: ctx.accounts.stake.miner.key(),
        owner: mm.owner.key(),
        amount,
    });

    Ok(())
}

/// Unstakes all of a [crate::MergeMiner]'s replica tokens for a [quarry_mine::Miner].
pub fn unstake_all_replica_miner(ctx: Context<QuarryStakeReplica>) -> ProgramResult {
    // pre-instruction checks
    let pre_replica_mint_supply = ctx.accounts.replica_mint.supply;
    invariant!(
        ctx.accounts.stake.miner.balance == ctx.accounts.stake.miner_vault.amount,
        "replica miner balance should equal miner_vault balance"
    );

    // Check to see if the merge miner is fully collateralized after the withdraw
    let mm = &ctx.accounts.stake.mm;

    // Unstake and burn all replica tokens
    let amount = mm.unstake_all_and_burn_replica_miner(ctx.accounts)?;

    // Update replica balance
    let mm = &mut ctx.accounts.stake.mm;
    mm.replica_balance = unwrap_int!(mm.replica_balance.checked_sub(amount));
    let pool = &mut ctx.accounts.stake.pool;
    pool.total_replica_balance = unwrap_int!(pool.total_replica_balance.checked_sub(amount));

    emit!(UnstakeReplicaEvent {
        pool: pool.key(),
        mm: mm.key(),
        miner: ctx.accounts.stake.miner.key(),
        owner: mm.owner.key(),
        amount,
    });

    // post-instruction checks
    post_unstake_replica_miner(ctx, pre_replica_mint_supply, amount)?;

    Ok(())
}

/// Withdraws tokens from the [MergeMiner].
pub fn withdraw_tokens(ctx: Context<WithdrawTokens>) -> ProgramResult {
    // skip withdrawal if there is nothing to claim
    if ctx.accounts.mm_token_account.amount == 0 {
        return Ok(());
    }
    let initial_balance = ctx.accounts.token_destination.amount;

    let mm = &ctx.accounts.mm;
    let event = mm.withdraw_tokens(ctx.accounts)?;

    ctx.accounts.mm_token_account.reload()?;
    invariant!(
        ctx.accounts.mm_token_account.amount == 0,
        "balance not empty"
    );

    ctx.accounts.token_destination.reload()?;
    let expected_dest_balance = unwrap_int!(initial_balance.checked_add(event.amount));
    invariant!(
        ctx.accounts.token_destination.amount == expected_dest_balance,
        "withdraw result invalid"
    );

    emit!(event);
    Ok(())
}

/// Checks run after [crate::quarry_merge_mine::unstake_replica_miner].
fn post_unstake_replica_miner(
    ctx: Context<QuarryStakeReplica>,
    pre_replica_mint_supply: u64,
    burn_amount: u64,
) -> ProgramResult {
    ctx.accounts.stake.miner.reload()?;
    ctx.accounts.stake.miner_vault.reload()?;
    ctx.accounts.replica_mint.reload()?;
    ctx.accounts.replica_mint_token_account.reload()?;
    invariant!(
        unwrap_int!(pre_replica_mint_supply.checked_sub(ctx.accounts.replica_mint.supply))
            == burn_amount,
        "supply increase should equal the stake amount"
    );
    invariant!(
        ctx.accounts.stake.miner.balance == 0,
        "replica miner balance should be zero"
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
