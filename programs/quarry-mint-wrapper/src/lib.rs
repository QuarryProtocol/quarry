//! Proxy program for interacting with the token mint.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

#[macro_use]
mod macros;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, TokenAccount};
use vipers::unwrap_int;
use vipers::validate::Validate;

mod account_validators;

declare_id!("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV");

#[program]
pub mod quarry_mint_wrapper {
    use super::*;

    /// --------------------------------
    /// [MintWrapper] instructions
    /// --------------------------------

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
        mint_wrapper.num_minters = 0;

        mint_wrapper.total_allowance = 0;
        mint_wrapper.total_minted = 0;

        emit!(NewMintWrapperEvent {
            mint_wrapper: mint_wrapper.key(),
            hard_cap,
            admin: ctx.accounts.admin.key(),
            token_mint: ctx.accounts.token_mint.key()
        });

        Ok(())
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

    /// --------------------------------
    /// [Minter] instructions
    /// --------------------------------

    /// Creates a new [Minter].
    #[access_control(ctx.accounts.validate())]
    pub fn new_minter(ctx: Context<NewMinter>, bump: u8) -> ProgramResult {
        let minter = &mut ctx.accounts.minter;

        minter.mint_wrapper = ctx.accounts.auth.mint_wrapper.key();
        minter.minter_authority = ctx.accounts.minter_authority.key();
        minter.bump = bump;

        let index = ctx.accounts.auth.mint_wrapper.num_minters;
        minter.index = index;

        // update num minters
        let mint_wrapper = &mut ctx.accounts.auth.mint_wrapper;
        mint_wrapper.num_minters = unwrap_int!(index.checked_add(1));

        minter.allowance = 0;
        minter.total_minted = 0;

        emit!(NewMinterEvent {
            mint_wrapper: minter.mint_wrapper,
            minter: minter.key(),
            index: minter.index,
            minter_authority: minter.minter_authority,
        });
        Ok(())
    }

    /// Updates a [Minter]'s allowance.
    #[access_control(ctx.accounts.validate())]
    pub fn minter_update(ctx: Context<MinterUpdate>, allowance: u64) -> ProgramResult {
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
    pub fn perform_mint(ctx: Context<PerformMint>, amount: u64) -> ProgramResult {
        let mint_wrapper = &ctx.accounts.mint_wrapper;
        let minter = &mut ctx.accounts.minter;
        require!(minter.allowance >= amount, MinterAllowanceExceeded);

        let new_supply = unwrap_int!(ctx.accounts.token_mint.supply.checked_add(amount));
        require!(new_supply <= mint_wrapper.hard_cap, HardcapExceeded);

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
        require!(new_supply == ctx.accounts.token_mint.supply, Unauthorized);

        emit!(MinterMintEvent {
            mint_wrapper: mint_wrapper.key(),
            minter: minter.key(),
            amount,
            destination: ctx.accounts.destination.key(),
        });
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
    pub base: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"MintWrapper",
            base.key().to_bytes().as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// Admin-to-be of the [MintWrapper].
    pub admin: UncheckedAccount<'info>,

    /// Token mint to mint.
    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    /// Token program.
    pub token_program: Program<'info, Token>,

    /// Payer.
    pub payer: UncheckedAccount<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

/// Adds a minter.
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct NewMinter<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,

    /// Account to authorize as a minter.
    pub minter_authority: UncheckedAccount<'info>,

    /// Information about the minter.
    #[account(
        init,
        seeds = [
            b"MintWrapperMinter",
            auth.mint_wrapper.key().to_bytes().as_ref(),
            minter_authority.key().to_bytes().as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub minter: Account<'info, Minter>,

    /// Payer for creating the minter.
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

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

/// --------------------------------
/// Account structs
/// --------------------------------

#[derive(Accounts)]
pub struct OnlyAdmin<'info> {
    /// The mint wrapper.
    #[account(mut)]
    pub mint_wrapper: Account<'info, MintWrapper>,
    pub admin: Signer<'info>,
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
    /// Number of [Minter]s.
    pub num_minters: u64,

    /// Total allowance outstanding.
    pub total_allowance: u64,
    /// Total amount of tokens minted through the [MintWrapper].
    pub total_minted: u64,
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
    /// Bump seed.
    pub bump: u8,

    /// Auto-incrementing index of the [Minter].
    pub index: u64,

    /// Limit of number of tokens that this [Minter] can mint.
    pub allowance: u64,
    /// Cumulative sum of the number of tokens ever minted by this [Minter].
    pub total_minted: u64,
}

/// --------------------------------
/// Events
/// --------------------------------

/// Triggered when a [MintWrapper] is created.
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

/// Triggered when a [MintWrapper]'s admin is proposed.
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

/// Triggered when a [MintWrapper]'s admin is transferred.
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

/// Triggered when a [Minter] is created.
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

/// Triggered when a [Minter]'s allowance is updated.
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

/// Triggered when a [Minter] performs a mint.
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
