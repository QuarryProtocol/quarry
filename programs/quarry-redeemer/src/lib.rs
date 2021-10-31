//! Program for redeeming IOU tokens for an underlying token.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{Mint, TokenAccount};
use vipers::invariant;
use vipers::unwrap_int;
use vipers::validate::Validate;

mod account_validators;
mod macros;
mod redeem_cpi;

declare_id!("QRDxhMw1P2NEfiw5mYXG79bwfgHTdasY2xNP76XSea9");

/// Quarry Redeemer program.
#[program]
pub mod quarry_redeemer {
    use super::*;

    /// Creates a new [Redeemer].
    #[access_control(ctx.accounts.validate())]
    pub fn create_redeemer(ctx: Context<CreateRedeemer>, bump: u8) -> ProgramResult {
        let redeemer = &mut ctx.accounts.redeemer;
        redeemer.iou_mint = ctx.accounts.iou_mint.key();
        redeemer.redemption_mint = ctx.accounts.redemption_mint.key();
        redeemer.bump = bump;

        redeemer.total_tokens_redeemed = 0;
        Ok(())
    }

    /// Redeems some of a user's tokens from the redemption vault.
    #[access_control(ctx.accounts.validate())]
    pub fn redeem_tokens(ctx: Context<RedeemTokens>, amount: u64) -> ProgramResult {
        invariant!(
            amount <= ctx.accounts.iou_source.amount,
            "insufficient iou_source balance"
        );
        invariant!(
            amount <= ctx.accounts.redemption_vault.amount,
            "insufficient redemption_vault balance"
        );

        ctx.accounts.burn_iou_tokens(amount)?;
        ctx.accounts.transfer_redemption_tokens(amount)?;

        let redeemer = &mut ctx.accounts.redeemer;
        redeemer.total_tokens_redeemed =
            unwrap_int!(redeemer.total_tokens_redeemed.checked_add(amount));

        let redeemer = &ctx.accounts.redeemer;
        emit!(RedeemTokensEvent {
            user: ctx.accounts.source_authority.key(),
            iou_mint: redeemer.iou_mint,
            redemption_mint: redeemer.redemption_mint,
            amount,
            timestamp: Clock::get()?.unix_timestamp
        });

        Ok(())
    }

    /// Redeems all of a user's tokens against the redemption vault.
    pub fn redeem_all_tokens(ctx: Context<RedeemTokens>) -> ProgramResult {
        let amount = ctx.accounts.iou_source.amount;
        redeem_tokens(ctx, amount)
    }
}

// --------------------------------
// Accounts
// --------------------------------

/// Redeemer state
#[account]
#[derive(Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Redeemer {
    /// [Mint] of the IOU token.
    pub iou_mint: Pubkey,
    /// [Mint] of the token to redeem.
    pub redemption_mint: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// Lifetime number of IOU tokens redeemed for redemption tokens.
    pub total_tokens_redeemed: u64,
}

// --------------------------------
// Instructions
// --------------------------------

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateRedeemer<'info> {
    /// Redeemer PDA.
    #[account(
        init,
        seeds = [
            b"Redeemer".as_ref(),
            iou_mint.to_account_info().key.as_ref(),
            redemption_mint.to_account_info().key.as_ref()
        ],
        bump = bump,
        payer = payer
    )]
    pub redeemer: Account<'info, Redeemer>,
    /// [Mint] of the IOU token.
    pub iou_mint: Account<'info, Mint>,
    /// [Mint] of the redemption token.
    pub redemption_mint: Account<'info, Mint>,
    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,
    /// [System] program.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RedeemTokens<'info> {
    /// Redeemer PDA.
    #[account(mut)]
    pub redeemer: Account<'info, Redeemer>,

    /// Authority of the source of the redeemed tokens.
    pub source_authority: Signer<'info>,
    /// [Mint] of the IOU token.
    #[account(mut)]
    pub iou_mint: Account<'info, Mint>,
    /// Source of the IOU tokens.
    #[account(mut)]
    pub iou_source: Account<'info, TokenAccount>,

    /// [TokenAccount] holding the [Redeemer]'s redemption tokens.
    #[account(mut)]
    pub redemption_vault: Account<'info, TokenAccount>,
    /// Destination of the IOU tokens.
    #[account(mut)]
    pub redemption_destination: Account<'info, TokenAccount>,

    /// The spl_token program corresponding to [Token].
    pub token_program: Program<'info, Token>,
}

// --------------------------------
// Events
// --------------------------------

#[event]
/// Emitted when tokens are redeemed.
pub struct RedeemTokensEvent {
    /// User which redeemed.
    #[index]
    pub user: Pubkey,
    /// IOU
    pub iou_mint: Pubkey,
    /// Redemption mint
    pub redemption_mint: Pubkey,
    /// Amount of tokens
    pub amount: u64,
    /// When the tokens were redeemed.
    pub timestamp: i64,
}

/// Errors
#[error]
pub enum ErrorCode {
    #[msg("Unauthorized.")]
    Unauthorized,
}
