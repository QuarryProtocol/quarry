//! Implementation of the [crate::quarry_mine::rescue_tokens] instruction.

use crate::*;

/// Handler for the [crate::quarry_mine::rescue_tokens] instruction.
pub fn handler(ctx: Context<RescueTokens>) -> Result<()> {
    let seeds = gen_miner_signer_seeds!(ctx.accounts.miner);
    let signer_seeds = &[&seeds[..]];

    // Transfer the tokens to the owner of the miner.
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.miner_token_account.to_account_info(),
                to: ctx.accounts.destination_token_account.to_account_info(),
                authority: ctx.accounts.miner.to_account_info(),
            },
            signer_seeds,
        ),
        ctx.accounts.miner_token_account.amount,
    )?;

    Ok(())
}

/// Accounts for the [crate::quarry_mine::rescue_tokens] instruction.
#[derive(Accounts)]
pub struct RescueTokens<'info> {
    /// Miner holding tokens.
    pub miner: Account<'info, Miner>,
    /// Miner authority (i.e. the user).
    pub authority: Signer<'info>,
    /// [TokenAccount] to withdraw tokens from.
    #[account(mut)]
    pub miner_token_account: Account<'info, TokenAccount>,
    /// [TokenAccount] to withdraw tokens into.
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    /// The SPL [token] program.
    pub token_program: Program<'info, token::Token>,
}

impl<'info> Validate<'info> for RescueTokens<'info> {
    fn validate(&self) -> Result<()> {
        // only callable by miner authority
        assert_keys_eq!(self.miner.authority, self.authority);

        // miner token vault should be completely unrelated to all accounts
        assert_keys_neq!(self.miner.token_vault_key, self.miner_token_account);
        assert_keys_neq!(self.miner.token_vault_key, self.destination_token_account);

        // miner token vault should be owned by the miner
        assert_keys_eq!(self.miner_token_account.owner, self.miner);

        // ensure correct mint
        assert_keys_eq!(
            self.miner_token_account.mint,
            self.destination_token_account.mint
        );
        Ok(())
    }
}
