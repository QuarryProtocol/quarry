//! Account conversions

use crate::{
    ClaimRewards, InitMiner, QuarryStake, QuarryStakePrimary, QuarryStakeReplica, WithdrawTokens,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

impl<'info> InitMiner<'info> {
    /// Conversion.
    pub fn to_create_miner_accounts(&self) -> quarry_mine::cpi::accounts::CreateMiner<'info> {
        quarry_mine::cpi::accounts::CreateMiner {
            authority: self.mm.to_account_info(),
            miner: self.miner.to_account_info(),
            quarry: self.quarry.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
            system_program: self.system_program.to_account_info(),
            payer: self.payer.to_account_info(),
            token_mint: self.token_mint.to_account_info(),
            miner_vault: self.miner_vault.to_account_info(),
            token_program: self.token_program.to_account_info(),
        }
    }
}

impl<'info> ClaimRewards<'info> {
    /// Conversion.
    pub fn to_claim_rewards_accounts(&self) -> quarry_mine::cpi::accounts::ClaimRewards<'info> {
        quarry_mine::cpi::accounts::ClaimRewards {
            mint_wrapper: self.mint_wrapper.to_account_info(),
            claim_fee_token_account: self.claim_fee_token_account.to_account_info(),
            mint_wrapper_program: self.mint_wrapper_program.to_account_info(),
            minter: self.minter.to_account_info(),
            rewards_token_mint: self.rewards_token_mint.to_account_info(),
            rewards_token_account: self.rewards_token_account.to_account_info(),
            stake: self.stake.gen_user_claim(),
        }
    }

    /// Conversion.
    pub fn to_user_stake_accounts(&self) -> quarry_mine::cpi::accounts::UserStake<'info> {
        self.stake.gen_user_stake(&self.stake_token_account)
    }
}

impl<'info> QuarryStake<'info> {
    /// Generates the [quarry_mine::UserStake] accounts.
    fn gen_user_claim(&self) -> quarry_mine::cpi::accounts::UserClaim<'info> {
        quarry_mine::cpi::accounts::UserClaim {
            authority: self.mm.to_account_info(),
            miner: self.miner.to_account_info(),
            quarry: self.quarry.to_account_info(),
            token_program: self.token_program.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
            unused_miner_vault: self.unused_account.to_account_info(),
            unused_token_account: self.unused_account.to_account_info(),
        }
    }

    /// Generates the [quarry_mine::UserStake] accounts.
    fn gen_user_stake(
        &self,
        token_account: &Account<'info, TokenAccount>,
    ) -> quarry_mine::cpi::accounts::UserStake<'info> {
        quarry_mine::cpi::accounts::UserStake {
            authority: self.mm.to_account_info(),
            miner: self.miner.to_account_info(),
            quarry: self.quarry.to_account_info(),
            miner_vault: self.miner_vault.to_account_info(),
            token_account: token_account.to_account_info(),
            token_program: self.token_program.to_account_info(),
            rewarder: self.rewarder.to_account_info(),
        }
    }
}

impl<'info> QuarryStakeReplica<'info> {
    /// Conversion.
    pub fn to_user_stake_accounts(&self) -> quarry_mine::cpi::accounts::UserStake<'info> {
        self.stake.gen_user_stake(&self.replica_mint_token_account)
    }

    /// Generates the accounts for minting replica tokens into a pool.
    pub fn to_mint_accounts(&self) -> token::MintTo<'info> {
        token::MintTo {
            mint: self.replica_mint.to_account_info(),
            to: self.replica_mint_token_account.to_account_info(),
            authority: self.stake.pool.to_account_info(),
        }
    }

    /// Generates the burn accounts for burning the replica tokens.
    pub fn to_burn_accounts(&self) -> token::Burn<'info> {
        token::Burn {
            mint: self.replica_mint.to_account_info(),
            to: self.replica_mint_token_account.to_account_info(),
            authority: self.stake.mm.to_account_info(),
        }
    }
}

impl<'info> QuarryStakePrimary<'info> {
    /// Conversion.
    pub fn to_user_stake_accounts(&self) -> quarry_mine::cpi::accounts::UserStake<'info> {
        self.stake.gen_user_stake(&self.mm_primary_token_account)
    }
}

impl<'info> WithdrawTokens<'info> {
    /// Conversion.
    pub fn to_transfer_accounts(&self) -> Transfer<'info> {
        Transfer {
            from: self.mm_token_account.to_account_info(),
            to: self.token_destination.to_account_info(),
            authority: self.mm.to_account_info(),
        }
    }
}
