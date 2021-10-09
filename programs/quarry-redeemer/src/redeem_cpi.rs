use crate::{gen_redeemer_signer_seeds, RedeemTokens};
use anchor_lang::prelude::*;
use anchor_spl::token;

impl<'info> RedeemTokens<'info> {
    /// Burn IOU tokens from source account.
    pub fn burn_iou_tokens(&self, amount: u64) -> ProgramResult {
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            token::Burn {
                mint: self.iou_mint.to_account_info(),
                to: self.iou_source.to_account_info(),
                authority: self.source_authority.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)
    }

    /// Transfer redemption tokens from the redemption vault to the user.
    pub fn transfer_redemption_tokens(&self, amount: u64) -> ProgramResult {
        let seeds = gen_redeemer_signer_seeds!(self.redeemer);
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            token::Transfer {
                from: self.redemption_vault.to_account_info(),
                to: self.redemption_destination.to_account_info(),
                authority: self.redeemer.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount)
    }
}
