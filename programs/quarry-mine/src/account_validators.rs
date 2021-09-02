//! Validations for various accounts.

use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token;
use vipers::validate::Validate;
use vipers::{assert_ata, assert_keys, assert_owner, assert_program};

use crate::addresses;
use crate::{
    AcceptAuthority, ClaimRewards, CreateMiner, CreateQuarry, ExtractFees,
    MutableRewarderWithAuthority, MutableRewarderWithPauseAuthority, NewRewarder,
    ReadOnlyRewarderWithAuthority, SetAnnualRewards, SetFamine, SetPauseAuthority, SetRewardsShare,
    TransferAuthority, UpdateQuarryRewards, UserStake,
};

/// --------------------------------
/// Rewarder Functions
/// --------------------------------

impl<'info> Validate<'info> for NewRewarder<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.base.is_signer, Unauthorized);

        assert_ata!(
            self.claim_fee_token_account,
            self.rewarder,
            self.rewards_token_mint
        );

        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        assert_keys!(
            self.mint_wrapper.token_mint,
            self.rewards_token_mint,
            "rewards token mint"
        );

        assert_owner!(self.mint_wrapper, quarry_mint_wrapper::ID, "mint_wrapper");
        assert_owner!(self.rewards_token_mint, token::ID, "rewards_token_mint");
        assert_owner!(
            self.claim_fee_token_account,
            token::ID,
            "claim_fee_token_account"
        );

        Ok(())
    }
}

impl<'info> Validate<'info> for SetPauseAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        require!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

impl<'info> Validate<'info> for MutableRewarderWithPauseAuthority<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.pause_authority.is_signer, Unauthorized);
        assert_keys!(
            self.rewarder.pause_authority,
            self.pause_authority,
            "pause_authority"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for TransferAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
        require!(self.authority.is_signer, Unauthorized);
        assert_keys!(self.authority, self.rewarder.authority);
        Ok(())
    }
}

impl<'info> Validate<'info> for AcceptAuthority<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
        require!(
            self.rewarder.pending_authority != Pubkey::default(),
            PendingAuthorityNotSet
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for SetAnnualRewards<'info> {
    /// Validates the [Rewarder] is correct.
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        require!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}

/// --------------------------------
/// Quarry functions
/// --------------------------------

impl<'info> Validate<'info> for CreateQuarry<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        require!(!self.auth.rewarder.is_paused, Paused);
        assert_program!(self.system_program, SYSTEM_PROGRAM_ID);
        assert_owner!(self.token_mint, token::ID, "token_mint");
        Ok(())
    }
}

impl<'info> Validate<'info> for SetRewardsShare<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        require!(!self.auth.rewarder.is_paused, Paused);
        assert_keys!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for SetFamine<'info> {
    fn validate(&self) -> ProgramResult {
        self.auth.validate()?;
        require!(!self.auth.rewarder.is_paused, Paused);
        assert_keys!(self.quarry.rewarder_key, self.auth.rewarder, "rewarder");
        Ok(())
    }
}

impl<'info> Validate<'info> for UpdateQuarryRewards<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
        assert_keys!(self.quarry.rewarder_key, self.rewarder, "rewarder");
        Ok(())
    }
}

/// --------------------------------
/// Miner functions
/// --------------------------------

impl<'info> Validate<'info> for CreateMiner<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
        assert_ata!(self.miner_vault, self.miner, self.token_mint, "miner vault");
        assert_keys!(self.miner_vault.owner, self.miner, "miner vault owner");
        assert_keys!(self.miner_vault.mint, self.token_mint, "miner vault mint");
        assert_ata!(self.miner_vault, self.miner, self.token_mint, "miner vault");
        assert_keys!(self.quarry.rewarder_key, self.rewarder, "rewarder");

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
        require!(!self.stake.rewarder.is_paused, Paused);

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

        // claim_fee_token_account validate
        assert_keys!(
            self.claim_fee_token_account,
            self.stake.rewarder.claim_fee_token_account,
            "claim_fee_token_account"
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

impl<'info> Validate<'info> for UserStake<'info> {
    /// Validates the UserStake.
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
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

impl<'info> Validate<'info> for ExtractFees<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.rewarder.is_paused, Paused);
        assert_ata!(
            self.claim_fee_token_account,
            self.rewarder,
            self.rewarder.rewards_token_mint
        );

        assert_program!(self.token_program, TOKEN_PROGRAM_ID);

        assert_keys!(
            self.claim_fee_token_account.mint,
            self.rewarder.rewards_token_mint,
            "claim_fee_token_account.mint"
        );
        assert_keys!(
            self.fee_to_token_account.mint,
            self.rewarder.rewards_token_mint,
            "fee_to_token_account.mint"
        );
        assert_keys!(
            self.fee_to_token_account.owner,
            addresses::FEE_TO,
            "fee_to_token_account.owner"
        );
        assert_ata!(
            self.fee_to_token_account,
            addresses::FEE_TO,
            self.rewarder.rewards_token_mint,
            "fee ata"
        );

        assert_owner!(
            self.claim_fee_token_account,
            token::ID,
            "claim_fee_token_account"
        );
        assert_owner!(self.fee_to_token_account, token::ID, "fee_to_token_account");

        Ok(())
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
        require!(self.authority.is_signer, Unauthorized);
        assert_keys!(self.authority, self.rewarder.authority);
        Ok(())
    }
}
