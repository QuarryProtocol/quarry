//! Validations for various accounts.

use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token;
use vipers::validate::Validate;
use vipers::{assert_ata, assert_keys, assert_owner, assert_program};

use crate::AcceptAuthority;
use crate::ClaimRewards;
use crate::CreateMiner;
use crate::CreateQuarry;
use crate::MutableRewarderWithAuthority;
use crate::NewRewarder;
use crate::ReadOnlyRewarderWithAuthority;
use crate::SetDailyRewards;
use crate::SetFamine;
use crate::SetRewardsShare;
use crate::TransferAuthority;
use crate::UpdateQuarryRewards;
use crate::UserStake;

impl<'info> Validate<'info> for NewRewarder<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.base.is_signer, Unauthorized);

        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        assert_keys!(
            self.mint_wrapper_program,
            quarry_mint_wrapper::ID,
            "mint wrapper"
        );
        assert_keys!(
            self.mint_wrapper.token_mint,
            self.rewards_token_mint,
            "rewards token mint"
        );

        assert_owner!(self.mint_wrapper, quarry_mint_wrapper::ID, "mint_wrapper");
        assert_owner!(self.rewards_token_mint, token::ID, "rewards_token_mint");

        Ok(())
    }
}

impl<'info> Validate<'info> for CreateQuarry<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        assert_owner!(self.token_mint, token::ID, "token_mint");
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRewardsShare<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        assert_keys!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for SetFamine<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        assert_keys!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for CreateMiner<'info> {
    fn validate(&self) -> ProgramResult {
        assert_ata!(self.miner_vault, self.miner, self.token_mint, "miner vault");
        assert_keys!(self.miner_vault.owner, self.miner, "miner vault owner");
        assert_keys!(self.miner_vault.mint, self.token_mint, "miner vault mint");
        assert_ata!(self.miner_vault, self.miner, self.token_mint, "miner vault");

        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        assert_program!(self.token_program, TOKEN_PROGRAM_ID);

        assert_owner!(self.token_mint, token::ID, "token_mint");
        assert_owner!(self.miner_vault, token::ID, "miner_vault");

        Ok(())
    }
}

impl<'info> Validate<'info> for ClaimRewards<'info> {
    /// Validates a [ClaimRewards] accounts struct.
    fn validate(&self) -> ProgramResult {
        self.stake.validate()?;

        // mint_wrapper_program validate
        assert_keys!(
            self.mint_wrapper_program,
            quarry_mint_wrapper::ID,
            "mint wrapper program"
        );

        // minter validate
        assert_keys!(
            self.minter.minter_authority,
            self.stake.rewarder,
            "rewarder"
        );

        // rewards_token_mint validate
        assert_keys!(
            self.rewards_token_mint,
            self.stake.rewarder.rewards_token_mint,
            "rewards token mint",
        );
        assert_keys!(
            self.rewards_token_mint,
            self.rewards_token_account.mint,
            "rewards token account mint",
        );
        assert_keys!(
            self.rewards_token_mint,
            self.mint_wrapper.token_mint,
            "mint wrapper mint",
        );
        assert_keys!(
            self.rewards_token_mint.mint_authority.unwrap(),
            self.mint_wrapper,
            "mint wrapper",
        );

        assert_owner!(self.mint_wrapper, quarry_mint_wrapper::ID, "mint_wrapper");
        assert_owner!(self.minter, quarry_mint_wrapper::ID, "minter");
        assert_owner!(self.rewards_token_mint, token::ID, "rewards_token_mint");
        assert_owner!(
            self.rewards_token_account,
            token::ID,
            "rewards_token_account"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        self.rewarder.only_owner(&self.authority)
    }
}

impl<'info> Validate<'info> for AcceptAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        require!(
            self.rewarder.pending_authority != Pubkey::default(),
            PendingAuthorityNotSet
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for SetDailyRewards<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        self.auth.validate()
    }
}

impl<'info> Validate<'info> for MutableRewarderWithAuthority<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.authority.is_signer, Unauthorized);
        assert_keys!(self.rewarder.authority, self.authority, "authority");
        Ok(())
    }
}

impl<'info> Validate<'info> for ReadOnlyRewarderWithAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        self.rewarder.only_owner(&self.authority)
    }
}

impl<'info> Validate<'info> for UserStake<'info> {
    /// Validates the UserStake.
    fn validate(&self) -> ProgramResult {
        // authority
        require!(self.authority.is_signer, Unauthorized);
        assert_keys!(self.authority, self.miner.authority, "miner authority");

        // quarry
        assert_keys!(self.miner.quarry_key, self.quarry.key(), "quarry");

        // miner_vault
        assert_keys!(self.miner.token_vault_key, self.miner_vault, "miner vault");
        assert_keys!(
            self.miner_vault.mint,
            self.quarry.token_mint_key,
            "vault mint"
        );
        assert_keys!(self.miner_vault.owner, self.miner, "vault owner");

        // token_account
        assert_keys!(
            self.token_account.mint,
            self.quarry.token_mint_key,
            "token mint"
        );

        // rewarder
        assert_keys!(self.quarry.rewarder_key, self.rewarder, "rewarder");

        assert_program!(self.token_program, TOKEN_PROGRAM_ID);

        assert_owner!(self.miner_vault, token::ID, "miner_vault");
        assert_owner!(self.token_account, token::ID, "token_account");

        Ok(())
    }
}

impl<'info> Validate<'info> for UpdateQuarryRewards<'info> {
    /// Validates a [ClaimRewards] accounts struct.
    fn validate(&self) -> ProgramResult {
        assert_keys!(self.quarry.rewarder_key, self.rewarder, "rewarder");

        Ok(())
    }
}
