use crate::*;

pub fn handler(ctx: Context<NewRewarder>) -> Result<()> {
    let rewarder = &mut ctx.accounts.rewarder;

    rewarder.base = ctx.accounts.base.key();
    rewarder.bump = unwrap_bump!(ctx, "rewarder");

    rewarder.authority = ctx.accounts.initial_authority.key();
    rewarder.pending_authority = Pubkey::default();

    rewarder.annual_rewards_rate = 0;
    rewarder.num_quarries = 0;
    rewarder.total_rewards_shares = 0;
    rewarder.mint_wrapper = ctx.accounts.mint_wrapper.key();

    rewarder.rewards_token_mint = ctx.accounts.rewards_token_mint.key();

    rewarder.claim_fee_token_account = ctx.accounts.claim_fee_token_account.key();
    rewarder.max_claim_fee_millibps = DEFAULT_CLAIM_FEE_MILLIBPS;

    rewarder.pause_authority = Pubkey::default();
    rewarder.is_paused = false;

    let current_ts = Clock::get()?.unix_timestamp;
    emit!(NewRewarderEvent {
        authority: rewarder.authority,
        timestamp: current_ts,
    });

    Ok(())
}

/// Accounts for [quarry_mine::new_rewarder].
#[derive(Accounts)]
pub struct NewRewarder<'info> {
    /// Base. Arbitrary key.
    pub base: Signer<'info>,

    /// [Rewarder] of mines.
    #[account(
        init,
        seeds = [
            b"Rewarder".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Rewarder::LEN
    )]
    pub rewarder: Account<'info, Rewarder>,

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
