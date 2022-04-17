use crate::*;

pub fn handler(ctx: Context<NewRewarder>) -> Result<()> {
    msg!("pt 2");
    execute_ix_handler(
        ctx.program_id,
        vec![
            ctx.accounts.base.to_account_info(),
            ctx.accounts.rewarder.to_account_info(),
            ctx.accounts.initial_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.mint_wrapper.to_account_info(),
            ctx.accounts.rewards_token_mint.to_account_info(),
            ctx.accounts.claim_fee_token_account.to_account_info(),
        ],
        crate::quarry_mine::new_rewarder_v2,
    )
}

/// Accounts for [quarry_mine::new_rewarder].
#[derive(Accounts)]
pub struct NewRewarder<'info> {
    /// Base. Arbitrary key.
    pub base: Signer<'info>,

    /// [Rewarder] of mines.
    #[account(mut)]
    pub rewarder: SystemAccount<'info>,

    /// Initial authority of the rewarder.
    /// CHECK: OK
    pub initial_authority: UncheckedAccount<'info>,

    /// Payer of the [Rewarder] initialization.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,

    /// Unused variable that held the [Clock]. Placeholder.
    /// CHECK: OK
    pub unused_account: UncheckedAccount<'info>,

    /// Mint wrapper.
    pub mint_wrapper: Account<'info, quarry_mint_wrapper::MintWrapper>,

    /// Rewards token mint.
    pub rewards_token_mint: Account<'info, Mint>,

    /// Token account in which the rewards token fees are collected.
    pub claim_fee_token_account: Account<'info, TokenAccount>,
}

impl<'info> Validate<'info> for NewRewarder<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.base.is_signer, Unauthorized);

        assert_keys_eq!(self.mint_wrapper.token_mint, self.rewards_token_mint);
        assert_keys_eq!(
            self.rewards_token_mint.mint_authority.unwrap(),
            self.mint_wrapper
        );

        assert_keys_eq!(self.claim_fee_token_account.owner, self.rewarder);
        assert_keys_eq!(self.claim_fee_token_account.mint, self.rewards_token_mint);
        invariant!(self.claim_fee_token_account.delegate.is_none());
        invariant!(self.claim_fee_token_account.close_authority.is_none());

        Ok(())
    }
}

/// Emitted when a new [Rewarder] is created.
#[event]
pub struct NewRewarderEvent {
    /// Authority of the rewarder
    #[index]
    pub authority: Pubkey,
    /// When the event occurred.
    pub timestamp: i64,
}
