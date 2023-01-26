//! Implementation of the [crate::quarry_merge_mine::rescue_tokens] instruction.

use crate::*;
use anchor_spl::token;
use quarry_mine::{program::QuarryMine, Miner};

/// Handler for the [crate::quarry_merge_mine::rescue_tokens] instruction.
pub fn handler(ctx: Context<RescueTokens>) -> Result<()> {
    // Rescue tokens
    let seeds = gen_merge_miner_signer_seeds!(ctx.accounts.mm);
    let signer_seeds = &[&seeds[..]];
    quarry_mine::cpi::rescue_tokens(CpiContext::new_with_signer(
        ctx.accounts.quarry_mine_program.to_account_info(),
        quarry_mine::cpi::accounts::RescueTokens {
            miner: ctx.accounts.miner.to_account_info(),
            authority: ctx.accounts.mm.to_account_info(),
            miner_token_account: ctx.accounts.miner_token_account.to_account_info(),
            destination_token_account: ctx.accounts.destination_token_account.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds,
    ))
}

/// Accounts for the [crate::quarry_merge_mine::rescue_tokens] instruction.
#[derive(Accounts)]
pub struct RescueTokens<'info> {
    /// The [MergeMiner::owner].
    pub mm_owner: Signer<'info>,
    /// The [MergePool].
    pub merge_pool: Account<'info, MergePool>,
    /// The [MergeMiner] (also the [quarry_mine::Miner] authority).
    #[account(constraint = mm.pool == merge_pool.key() && mm.owner == mm_owner.key())]
    pub mm: Account<'info, MergeMiner>,

    /// Miner holding tokens (owned by the [MergeMiner]).
    pub miner: Account<'info, Miner>,
    /// [TokenAccount] to withdraw tokens from.
    #[account(mut)]
    pub miner_token_account: Account<'info, TokenAccount>,
    /// [TokenAccount] to withdraw tokens into.
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    /// The [quarry_mine] program.
    pub quarry_mine_program: Program<'info, QuarryMine>,
    /// The SPL [token] program.
    pub token_program: Program<'info, token::Token>,
}

impl<'info> Validate<'info> for RescueTokens<'info> {
    fn validate(&self) -> Result<()> {
        // only callable by merge miner authority
        assert_keys_eq!(self.mm_owner, self.mm.owner, Unauthorized);

        // merge pool of the merge miner
        assert_keys_eq!(self.merge_pool, self.mm.pool);

        // mm must be authority of the miner
        assert_keys_eq!(self.miner.authority, self.mm);

        // miner token vault should be completely unrelated to all accounts
        assert_keys_neq!(self.miner.token_vault_key, self.miner_token_account);
        assert_keys_neq!(self.miner.token_vault_key, self.destination_token_account);

        // don't allow withdraws back to the miner
        assert_keys_neq!(self.miner.token_vault_key, self.destination_token_account);

        // miner token vault should be owned by the miner
        assert_keys_eq!(self.miner_token_account.owner, self.miner);

        // ensure correct mint
        assert_keys_eq!(
            self.miner_token_account.mint,
            self.destination_token_account.mint
        );

        // cannot be a primary or replica mint
        let mint = self.miner_token_account.mint;
        assert_keys_neq!(self.merge_pool.primary_mint, mint);
        assert_keys_neq!(self.merge_pool.replica_mint, mint);

        Ok(())
    }
}
