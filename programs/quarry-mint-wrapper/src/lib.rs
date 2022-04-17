//! Proxy program for interacting with the token mint.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(deprecated)]

#[macro_use]
mod macros;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, TokenAccount};
use vipers::prelude::*;

mod account_validators;
mod instructions;
mod state;

use instructions::*;
pub use state::*;

declare_id!("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Quarry Mint Wrapper",
    project_url: "https://quarry.so",
    contacts: "email:team@quarry.so",
    policy: "https://github.com/QuarryProtocol/quarry/blob/master/SECURITY.md",

    source_code: "https://github.com/QuarryProtocol/quarry",
    auditors: "Quantstamp"
}

#[program]
pub mod quarry_mint_wrapper {
    use super::*;

    // --------------------------------
    // [MintWrapper] instructions
    // --------------------------------

    /// Creates a new [MintWrapper].
    #[deprecated(since = "5.0.0", note = "Use `new_wrapper_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn new_wrapper(ctx: Context<NewWrapper>, _bump: u8, hard_cap: u64) -> Result<()> {
        instructions::new_wrapper::handler(ctx, hard_cap)
    }

    /// Creates a new [MintWrapper].
    ///
    /// The V2 variant removes the need for supplying the bump.
    #[access_control(ctx.accounts.validate())]
    pub fn new_wrapper_v2(ctx: Context<NewWrapper>, hard_cap: u64) -> Result<()> {
        instructions::new_wrapper::handler(ctx, hard_cap)
    }

    /// Transfers admin to another account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_admin(ctx: Context<TransferAdmin>) -> Result<()> {
        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        mint_wrapper.pending_admin = ctx.accounts.next_admin.key();

        emit!(MintWrapperAdminProposeEvent {
            mint_wrapper: mint_wrapper.key(),
            current_admin: mint_wrapper.admin,
            pending_admin: mint_wrapper.pending_admin,
        });
        Ok(())
    }

    /// Accepts the new admin.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        let previous_admin = mint_wrapper.admin;
        mint_wrapper.admin = ctx.accounts.pending_admin.key();
        mint_wrapper.pending_admin = Pubkey::default();

        emit!(MintWrapperAdminUpdateEvent {
            mint_wrapper: mint_wrapper.key(),
            previous_admin,
            admin: mint_wrapper.admin,
        });
        Ok(())
    }

    // --------------------------------
    // [Minter] instructions
    // --------------------------------

    /// Creates a new [Minter].
    #[deprecated(since = "5.0.0", note = "Use `new_minter_v2` instead.")]
    #[access_control(ctx.accounts.validate())]
    pub fn new_minter(ctx: Context<NewMinter>, _bump: u8) -> Result<()> {
        instructions::new_minter::handler(ctx)
    }

    /// Creates a new [Minter].
    ///
    /// The V2 variant removes the need for supplying the bump.
    #[access_control(ctx.accounts.validate())]
    pub fn new_minter_v2(ctx: Context<NewMinter>) -> Result<()> {
        instructions::new_minter::handler(ctx)
    }

    /// Updates a [Minter]'s allowance.
    #[access_control(ctx.accounts.validate())]
    pub fn minter_update(ctx: Context<MinterUpdate>, allowance: u64) -> Result<()> {
        let minter = &mut ctx.accounts.minter;
        let previous_allowance = minter.allowance;
        minter.allowance = allowance;

        let mint_wrapper = &mut ctx.accounts.auth.mint_wrapper;
        mint_wrapper.total_allowance = unwrap_int!(mint_wrapper
            .total_allowance
            .checked_add(allowance)
            .and_then(|v| v.checked_sub(previous_allowance)));

        emit!(MinterAllowanceUpdateEvent {
            mint_wrapper: minter.mint_wrapper,
            minter: minter.key(),
            previous_allowance,
            allowance: minter.allowance,
        });
        Ok(())
    }

    /// Performs a mint.
    #[access_control(ctx.accounts.validate())]
    pub fn perform_mint(ctx: Context<PerformMint>, amount: u64) -> Result<()> {
        let mint_wrapper = &ctx.accounts.mint_wrapper;
        let minter = &mut ctx.accounts.minter;
        invariant!(minter.allowance >= amount, MinterAllowanceExceeded);

        let new_supply = unwrap_int!(ctx.accounts.token_mint.supply.checked_add(amount));
        invariant!(new_supply <= mint_wrapper.hard_cap, HardcapExceeded);

        minter.allowance = unwrap_int!(minter.allowance.checked_sub(amount));
        minter.total_minted = unwrap_int!(minter.total_minted.checked_add(amount));

        let seeds = gen_wrapper_signer_seeds!(mint_wrapper);
        let proxy_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.mint_wrapper.to_account_info(),
            },
            proxy_signer,
        );
        token::mint_to(cpi_ctx, amount)?;

        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        mint_wrapper.total_allowance =
            unwrap_int!(mint_wrapper.total_allowance.checked_sub(amount));
        mint_wrapper.total_minted = unwrap_int!(mint_wrapper.total_minted.checked_add(amount));

        // extra sanity checks
        ctx.accounts.token_mint.reload()?;
        invariant!(new_supply == ctx.accounts.token_mint.supply, Unauthorized);

        emit!(MinterMintEvent {
            mint_wrapper: mint_wrapper.key(),
            minter: minter.key(),
            amount,
            destination: ctx.accounts.destination.key(),
        });
        Ok(())
    }
}

// --------------------------------
// Instructions
// --------------------------------

/// Updates a minter.
#[derive(Accounts)]
pub struct MinterUpdate<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,
    /// Information about the minter.
    #[account(mut)]
    pub minter: Account<'info, Minter>,
}

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// The [MintWrapper].
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// The previous admin.
    pub admin: Signer<'info>,

    /// The next admin.
    /// CHECK: OK
    pub next_admin: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    /// The mint wrapper.
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// The new admin.
    pub pending_admin: Signer<'info>,
}

/// Accounts for the perform_mint instruction.
#[derive(Accounts, Clone)]
pub struct PerformMint<'info> {
    /// [MintWrapper].
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// [Minter]'s authority.
    pub minter_authority: Signer<'info>,

    /// Token [Mint].
    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    /// Destination [TokenAccount] for minted tokens.
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    /// [Minter] information.
    #[account(mut)]
    pub minter: Account<'info, Minter>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

// --------------------------------
// Account structs
// --------------------------------

/// Only an admin is allowed to use instructions containing this struct.
#[derive(Accounts)]
pub struct OnlyAdmin<'info> {
    /// The [MintWrapper].
    #[account(mut, has_one = admin @ ErrorCode::Unauthorized)]
    pub mint_wrapper: Account<'info, MintWrapper>,
    /// [MintWrapper::admin].
    pub admin: Signer<'info>,
}

// --------------------------------
// Events
// --------------------------------

/// Emitted when a [MintWrapper] is created.
#[event]
pub struct NewMintWrapperEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,

    /// Hard cap.
    pub hard_cap: u64,
    /// The admin.
    pub admin: Pubkey,
    /// The [Mint] of the token.
    pub token_mint: Pubkey,
}

/// Emitted when a [MintWrapper]'s admin is proposed.
#[event]
pub struct MintWrapperAdminProposeEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,

    /// The [MintWrapper]'s current admin.
    pub current_admin: Pubkey,
    /// The [MintWrapper]'s pending admin.
    pub pending_admin: Pubkey,
}

/// Emitted when a [MintWrapper]'s admin is transferred.
#[event]
pub struct MintWrapperAdminUpdateEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,

    /// The [MintWrapper]'s previous admin.
    pub previous_admin: Pubkey,
    /// The [MintWrapper]'s new admin.
    pub admin: Pubkey,
}

/// Emitted when a [Minter] is created.
#[event]
pub struct NewMinterEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,
    /// The [Minter].
    #[index]
    pub minter: Pubkey,

    /// The [Minter]'s index.
    pub index: u64,
    /// The [Minter]'s authority.
    pub minter_authority: Pubkey,
}

/// Emitted when a [Minter]'s allowance is updated.
#[event]
pub struct MinterAllowanceUpdateEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,
    /// The [Minter].
    #[index]
    pub minter: Pubkey,

    /// The [Minter]'s previous allowance.
    pub previous_allowance: u64,
    /// The [Minter]'s new allowance.
    pub allowance: u64,
}

/// Emitted when a [Minter] performs a mint.
#[event]
pub struct MinterMintEvent {
    /// The [MintWrapper].
    #[index]
    pub mint_wrapper: Pubkey,
    /// The [Minter].
    #[index]
    pub minter: Pubkey,

    /// Amount minted.
    pub amount: u64,
    /// Mint destination.
    pub destination: Pubkey,
}

/// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("Cannot mint over hard cap.")]
    HardcapExceeded,
    #[msg("Minter allowance exceeded.")]
    MinterAllowanceExceeded,
}
