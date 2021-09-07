//! Proxy program for interacting with the token mint.

#[macro_use]
mod macros;

use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token::{self, Mint, TokenAccount};
use vipers::validate::Validate;

mod account_validators;

solana_program::declare_id!("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV");

#[program]
pub mod quarry_mint_wrapper {
    use vipers::unwrap_int;

    use super::*;

    /// Creates a new [MintWrapper].
    #[access_control(ctx.accounts.validate())]
    pub fn new_wrapper(ctx: Context<NewWrapper>, bump: u8, hard_cap: u64) -> ProgramResult {
        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        mint_wrapper.base = ctx.accounts.base.key();
        mint_wrapper.bump = bump;
        mint_wrapper.hard_cap = hard_cap;
        mint_wrapper.admin = ctx.accounts.admin.key();
        mint_wrapper.pending_admin = Pubkey::default();
        mint_wrapper.token_mint = ctx.accounts.token_mint.key();

        Ok(())
    }

    /// Creates a new [Minter].
    #[access_control(ctx.accounts.validate())]
    pub fn new_minter(ctx: Context<NewMinter>, bump: u8) -> ProgramResult {
        let minter = &mut ctx.accounts.minter;

        minter.mint_wrapper = ctx.accounts.auth.mint_wrapper.key();
        minter.minter_authority = ctx.accounts.minter_authority.key();
        minter.bump = bump;
        minter.allowance = 0;
        Ok(())
    }

    /// Updates a [Minter]'s allowance.
    #[access_control(ctx.accounts.validate())]
    pub fn minter_update(ctx: Context<MinterUpdate>, allowance: u64) -> ProgramResult {
        let minter = &mut ctx.accounts.minter;
        minter.allowance = allowance;
        Ok(())
    }

    /// Transfers admin to another account.
    #[access_control(ctx.accounts.validate())]
    pub fn transfer_admin(ctx: Context<TransferAdmin>) -> Result<()> {
        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        mint_wrapper.pending_admin = ctx.accounts.next_admin.key();
        Ok(())
    }

    /// Accepts the new admin.
    #[access_control(ctx.accounts.validate())]
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        let mint_wrapper = &mut ctx.accounts.mint_wrapper;
        mint_wrapper.admin = ctx.accounts.pending_admin.key();
        mint_wrapper.pending_admin = Pubkey::default();
        Ok(())
    }

    /// Performs a mint.
    #[access_control(ctx.accounts.validate())]
    pub fn perform_mint(ctx: Context<PerformMint>, amount: u64) -> ProgramResult {
        let mint_wrapper = &ctx.accounts.mint_wrapper;
        let minter = &mut ctx.accounts.minter;
        require!(minter.allowance >= amount, MinterAllowanceExceeded);

        let new_supply = unwrap_int!(ctx.accounts.token_mint.supply.checked_add(amount));
        require!(new_supply <= mint_wrapper.hard_cap, HardcapExceeded);

        minter.allowance = unwrap_int!(minter.allowance.checked_sub(amount));

        let seeds = gen_wrapper_signer_seeds!(mint_wrapper);
        let proxy_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.clone(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.mint_wrapper.to_account_info(),
            },
            proxy_signer,
        );
        token::mint_to(cpi_ctx, amount)?;
        Ok(())
    }
}

/// --------------------------------
/// Instructions
/// --------------------------------

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct NewWrapper<'info> {
    /// Base account.
    #[account(signer)]
    pub base: AccountInfo<'info>,

    #[account(
        init,
        seeds = [
            b"MintWrapper",
            base.key().to_bytes().as_ref(),
            &[bump]
        ],
        payer = payer
    )]
    pub mint_wrapper: ProgramAccount<'info, MintWrapper>,

    /// Admin-to-be of the [MintWrapper].
    pub admin: AccountInfo<'info>,

    /// Token mint to mint.
    #[account(mut)]
    pub token_mint: CpiAccount<'info, Mint>,

    /// Token program.
    pub token_program: AccountInfo<'info>,

    /// Payer.
    pub payer: AccountInfo<'info>,

    /// System program.
    pub system_program: AccountInfo<'info>,
}

/// Adds a minter.
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct NewMinter<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,

    /// Account to authorize as a minter.
    pub minter_authority: AccountInfo<'info>,

    /// Information about the minter.
    #[account(
        init,
        seeds = [
            b"MintWrapperMinter",
            auth.mint_wrapper.key().to_bytes().as_ref(),
            minter_authority.key().to_bytes().as_ref(),
            &[bump]
        ],
        payer = payer
    )]
    pub minter: ProgramAccount<'info, Minter>,

    /// Payer for creating the minter.
    pub payer: AccountInfo<'info>,

    /// System program.
    pub system_program: AccountInfo<'info>,
}

/// Updates a minter.
#[derive(Accounts)]
pub struct MinterUpdate<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,
    /// Information about the minter.
    #[account(mut)]
    pub minter: ProgramAccount<'info, Minter>,
}

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// The mint wrapper.
    #[account(mut)]
    pub mint_wrapper: ProgramAccount<'info, MintWrapper>,

    /// The previous admin.
    #[account(signer)]
    pub admin: AccountInfo<'info>,

    /// The next admin.
    pub next_admin: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    /// The mint wrapper.
    #[account(mut)]
    pub mint_wrapper: ProgramAccount<'info, MintWrapper>,

    /// The new admin.
    #[account(signer)]
    pub pending_admin: AccountInfo<'info>,
}

/// Accounts for the perform_mint instruction.
#[derive(Accounts)]
pub struct PerformMint<'info> {
    /// Mint wrapper.
    pub mint_wrapper: ProgramAccount<'info, MintWrapper>,

    /// Minter.
    #[account(signer)]
    pub minter_authority: AccountInfo<'info>,

    /// Token mint.
    #[account(mut)]
    pub token_mint: CpiAccount<'info, Mint>,

    /// Destination account for minted tokens.
    #[account(mut)]
    pub destination: CpiAccount<'info, TokenAccount>,

    /// Minter information.
    #[account(mut)]
    pub minter: ProgramAccount<'info, Minter>,

    /// SPL Token program.
    pub token_program: AccountInfo<'info>,
}

/// --------------------------------
/// Account structs
/// --------------------------------

#[derive(Accounts)]
pub struct OnlyAdmin<'info> {
    /// The mint wrapper.
    pub mint_wrapper: ProgramAccount<'info, MintWrapper>,
    #[account(signer)]
    pub admin: AccountInfo<'info>,
}

/// --------------------------------
/// PDA structs
/// --------------------------------

/// Mint wrapper
///
/// ```
/// seeds = [
///     b"MintWrapper",
///     base.key().to_bytes().as_ref(),
///     &[bump]
/// ],
///
#[account]
#[derive(Default)]
pub struct MintWrapper {
    /// Base account.
    pub base: Pubkey,
    /// Bump for allowing the proxy mint authority to sign.
    pub bump: u8,
    /// Maximum number of tokens that can be issued.
    pub hard_cap: u64,

    /// Admin account.
    pub admin: Pubkey,
    /// Next admin account.
    pub pending_admin: Pubkey,

    /// Mint of the token.
    pub token_mint: Pubkey,
}

/// One who can mint.
///
/// ```
/// seeds = [
///     b"MintWrapperMinter",
///     auth.mint_wrapper.key().to_bytes().as_ref(),
///     minter_authority.key().to_bytes().as_ref(),
///     &[bump]
/// ],
/// ```
#[account]
#[derive(Default)]
pub struct Minter {
    /// The mint wrapper.
    pub mint_wrapper: Pubkey,
    /// Address that can mint.
    pub minter_authority: Pubkey,
    pub bump: u8,

    /// Limit of number of tokens that this minter can mint.
    pub allowance: u64,
}

/// --------------------------------
/// Error Codes
/// --------------------------------

#[error]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("Cannot mint over hard cap.")]
    HardcapExceeded,
    #[msg("Minter allowance exceeded.")]
    MinterAllowanceExceeded,
}
