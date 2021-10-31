//! Registry to help the frontend quickly locate all active quarries.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

use anchor_lang::prelude::*;
use quarry_mine::Quarry;
use quarry_mine::Rewarder;

mod account_validators;

declare_id!("QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc");

#[program]
pub mod quarry_registry {
    use vipers::validate::Validate;

    use super::*;

    /// Provisions a new registry for a [Rewarder].
    ///
    /// # Arguments
    ///
    /// * `max_quarries` - The maximum number of quarries that can be held in the registry.
    /// * `bump` - Bump seed.
    pub fn new_registry(ctx: Context<NewRegistry>, max_quarries: u16, bump: u8) -> ProgramResult {
        ctx.accounts.validate()?;
        let registry = &mut ctx.accounts.registry;
        registry.bump = bump;
        registry.rewarder = ctx.accounts.rewarder.key();
        registry
            .tokens
            .resize(max_quarries as usize, Pubkey::default());
        Ok(())
    }

    /// Synchronizes a [Quarry]'s token mint with the registry of its [Rewarder].
    pub fn sync_quarry(ctx: Context<SyncQuarry>) -> ProgramResult {
        ctx.accounts.validate()?;
        let quarry = &ctx.accounts.quarry;
        let registry = &mut ctx.accounts.registry;
        registry.tokens[quarry.index as usize] = quarry.token_mint_key;
        Ok(())
    }
}

/// Accounts for [quarry_registry::new_registry].
#[derive(Accounts)]
#[instruction(max_quarries: u16, bump: u8)]
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
        bump = bump,
        payer = payer,
        space = (8 + 1 + 32 + 32 * max_quarries + 100) as usize
    )]
    pub registry: Account<'info, Registry>,

    /// Payer of the [Registry] initialization.
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
