//! Registry to help the frontend quickly locate all active quarries.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

use anchor_lang::prelude::*;
use quarry_mine::{Quarry, Rewarder};
use vipers::prelude::*;

mod account_validators;

declare_id!("QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Quarry Registry",
    project_url: "https://quarry.so",
    contacts: "email:team@quarry.so",
    policy: "https://github.com/QuarryProtocol/quarry/blob/master/SECURITY.md",

    source_code: "https://github.com/QuarryProtocol/quarry",
    auditors: "Quantstamp"
}

/// Registry to help frontends quickly locate all active quarries.
#[program]
pub mod quarry_registry {

    use super::*;

    /// Provisions a new registry for a [Rewarder].
    ///
    /// # Arguments
    ///
    /// * `max_quarries` - The maximum number of quarries that can be held in the registry.
    /// * `bump` - Bump seed.
    pub fn new_registry(ctx: Context<NewRegistry>, max_quarries: u16, _bump: u8) -> Result<()> {
        ctx.accounts.validate()?;
        let registry = &mut ctx.accounts.registry;
        registry.bump = ctx.bumps.registry;
        registry.rewarder = ctx.accounts.rewarder.key();
        registry
            .tokens
            .resize(max_quarries as usize, Pubkey::default());
        Ok(())
    }

    /// Synchronizes a [Quarry]'s token mint with the registry of its [Rewarder].
    pub fn sync_quarry(ctx: Context<SyncQuarry>) -> Result<()> {
        ctx.accounts.validate()?;
        let quarry = &ctx.accounts.quarry;
        let registry = &mut ctx.accounts.registry;
        registry.tokens[quarry.index as usize] = quarry.token_mint_key;
        Ok(())
    }
}

/// Accounts for [quarry_registry::new_registry].
#[derive(Accounts)]
#[instruction(max_quarries: u16)]
pub struct NewRegistry<'info> {
    /// [Rewarder].
    pub rewarder: Account<'info, Rewarder>,

    /// [Rewarder] of mines.
    #[account(
        init,
        seeds = [
            b"QuarryRegistry".as_ref(),
            rewarder.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = (8 + 1 + 32 + 32 * max_quarries + 100) as usize
    )]
    pub registry: Account<'info, Registry>,

    /// Payer of the [Registry] initialization.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

/// Accounts for [quarry_registry::sync_quarry].
#[derive(Accounts)]
pub struct SyncQuarry<'info> {
    /// [Quarry] to sync.
    pub quarry: Account<'info, Quarry>,
    /// [Registry] to write to.
    #[account(mut)]
    pub registry: Account<'info, Registry>,
}

/// The [Registry] of all token mints associated with a [Rewarder].
#[account]
#[derive(Default, Debug)]
pub struct Registry {
    /// Bump seed
    pub bump: u8,
    /// Rewarder
    pub rewarder: Pubkey,
    /// Tokens
    pub tokens: Vec<Pubkey>,
}

impl Registry {
    /// Number of bytes a [Registry] takes up when serialized.
    pub fn byte_length(max_quarries: u16) -> usize {
        (1 + 32 + 4 + 32 * max_quarries) as usize
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::system_program;

    use super::*;

    #[test]
    fn test_registry_len() {
        let registry = Registry {
            tokens: vec![system_program::ID, system_program::ID],
            ..Default::default()
        };
        assert_eq!(
            registry.try_to_vec().unwrap().len(),
            Registry::byte_length(2)
        );
    }
}
