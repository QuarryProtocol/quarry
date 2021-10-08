//! Claim-related instructions.

use crate::{events::*, ClaimRewards};
use anchor_lang::prelude::*;
use vipers::*;

/// Claims [quarry_mine] rewards on behalf of the [MergeMiner].
pub fn claim_rewards(ctx: Context<ClaimRewards>) -> ProgramResult {
    let initial_balance = ctx.accounts.rewards_token_account.amount;

    let mm = &ctx.accounts.stake.mm;
    mm.claim_rewards(ctx.accounts)?;

    ctx.accounts.rewards_token_account.reload()?;
    let end_balance = ctx.accounts.rewards_token_account.amount;
    let amount = unwrap_int!(end_balance.checked_sub(initial_balance));

    emit!(ClaimEvent {
        pool: mm.pool.key(),
        mm: mm.key(),
        mint: ctx.accounts.rewards_token_account.mint,
        amount,
        initial_balance,
        end_balance,
    });

    Ok(())
}
