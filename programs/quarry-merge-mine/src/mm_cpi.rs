//! CPI calls made on behalf of a [crate::MergeMiner].
#![deny(clippy::integer_arithmetic, clippy::float_arithmetic)]

use crate::events::WithdrawTokensEvent;
use crate::{
    ClaimRewards, InitMiner, MergeMiner, QuarryStakePrimary, QuarryStakeReplica, WithdrawTokens,
};
use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token;
use vipers::unwrap_int;

impl MergeMiner {
    /// Initializes a [quarry_mine::Miner] for the [MergeMiner].
    pub fn init_miner(&self, init: &InitMiner, bump: u8) -> ProgramResult {
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            init.mine_program.to_account_info(),
            init.to_create_miner_accounts(),
            signer_seeds,
        );
        quarry_mine::cpi::create_miner(cpi_ctx, bump)
    }

    /// Stakes all available primary tokens owned by the [MergeMiner] into the primary miner.
    /// Returns the number of tokens deposited.
    pub fn stake_max_primary_miner(&self, stake: &QuarryStakePrimary) -> Result<u64, ProgramError> {
        let amount = stake.mm_primary_token_account.amount;
        // short circuit in case there is nothing to stake
        if amount == 0 {
            return Ok(0);
        }

        // Deposit tokens
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.mine_program.to_account_info(),
            stake.to_user_stake_accounts(),
            signer_seeds,
        );
        quarry_mine::cpi::stake_tokens(cpi_ctx, amount)?;
        Ok(amount)
    }

    /// Mints the maximum number of replica tokens for the [crate::MergeMiner],
    /// staking them into a [quarry_mine::Miner].
    pub fn stake_max_replica_miner(&self, stake: &QuarryStakeReplica) -> Result<u64, ProgramError> {
        // amount of tokens to mint
        // this is the current balance of primary tokens minus the miner's balance
        let amount = unwrap_int!(self.primary_balance.checked_sub(stake.stake.miner.balance));
        // short circuit in case there is nothing to stake
        if amount == 0 {
            return Ok(0);
        }

        // Mint replica tokens to stake in the pool
        let seeds = gen_pool_signer_seeds!(stake.stake.pool);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.mine_program.to_account_info(),
            stake.to_mint_accounts(),
            signer_seeds,
        );
        token::mint_to(cpi_ctx, amount)?;

        // Stake tokens
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.mine_program.to_account_info(),
            stake.to_user_stake_accounts(),
            signer_seeds,
        );
        quarry_mine::cpi::stake_tokens(cpi_ctx, amount)?;

        Ok(amount)
    }

    /// Unstakes from the primary miner.
    pub fn unstake_primary_miner(&self, stake: &QuarryStakePrimary, amount: u64) -> ProgramResult {
        require!(amount <= stake.stake.miner.balance, InsufficientBalance);

        // noop if there is nothing to usntake
        if amount == 0 {
            return Ok(());
        }

        // Withdraw tokens
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.mine_program.to_account_info(),
            stake.to_user_stake_accounts(),
            signer_seeds,
        );
        quarry_mine::cpi::withdraw_tokens(cpi_ctx, amount)
    }

    /// Unstake tokens from a replica [quarry_mine::Miner] and burns the replica tokens.
    pub fn unstake_all_and_burn_replica_miner(
        &self,
        stake: &QuarryStakeReplica,
    ) -> Result<u64, ProgramError> {
        let amount = stake.stake.miner.balance;
        // noop if there is nothing to unstake
        if amount == 0 {
            return Ok(0);
        }

        // Unstake tokens
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.mine_program.to_account_info(),
            stake.to_user_stake_accounts(),
            signer_seeds,
        );
        quarry_mine::cpi::withdraw_tokens(cpi_ctx, amount)?;

        // Burn replica tokens
        let seeds = gen_merge_miner_signer_seeds!(stake.stake.mm);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            stake.stake.token_program.to_account_info(),
            stake.to_burn_accounts(),
            signer_seeds,
        );
        token::burn(cpi_ctx, amount)?;

        Ok(amount)
    }

    /// Withdraws tokens from the [MergeMiner].
    pub fn withdraw_tokens(
        &self,
        withdraw: &WithdrawTokens,
    ) -> Result<WithdrawTokensEvent, ProgramError> {
        let amount = withdraw.mm_token_account.amount;
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];

        // transfer tokens to user
        token::transfer(
            CpiContext::new_with_signer(
                withdraw.token_program.to_account_info(),
                withdraw.to_transfer_accounts(),
                signer_seeds,
            ),
            amount,
        )?;

        Ok(WithdrawTokensEvent {
            pool: withdraw.pool.key(),
            mm: withdraw.mm.key(),
            owner: withdraw.owner.key(),
            mint: withdraw.mm_token_account.mint,
            amount,
        })
    }

    /// Claims [quarry_mine] rewards as the [MergeMiner].
    pub fn claim_rewards(&self, claim: &ClaimRewards) -> ProgramResult {
        let seeds = gen_merge_miner_signer_seeds!(self);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx: CpiContext<quarry_mine::cpi::accounts::ClaimRewards> =
            CpiContext::new_with_signer(
                claim.stake.mine_program.to_account_info(),
                claim.to_claim_rewards_accounts(),
                signer_seeds,
            );
        quarry_mine::cpi::claim_rewards(cpi_ctx)
    }
}
