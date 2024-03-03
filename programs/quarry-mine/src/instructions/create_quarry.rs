use crate::*;

pub fn handler(ctx: Context<CreateQuarry>) -> Result<()> {
    let rewarder = &mut ctx.accounts.auth.rewarder;
    // Update rewarder's quarry stats
    let index = rewarder.num_quarries;
    rewarder.num_quarries = unwrap_int!(rewarder.num_quarries.checked_add(1));

    let quarry = &mut ctx.accounts.quarry;
    quarry.bump = ctx.bumps.quarry;

    // Set quarry params
    quarry.index = index;
    quarry.famine_ts = i64::MAX;
    quarry.rewarder = rewarder.key();
    quarry.annual_rewards_rate = 0;
    quarry.rewards_share = 0;
    quarry.token_mint_decimals = ctx.accounts.token_mint.decimals;
    quarry.token_mint_key = ctx.accounts.token_mint.key();

    let current_ts = Clock::get()?.unix_timestamp;
    emit!(QuarryCreateEvent {
        token_mint: quarry.token_mint_key,
        timestamp: current_ts,
    });

    Ok(())
}

/// Accounts for [quarry_mine::create_quarry].
#[derive(Accounts)]
pub struct CreateQuarry<'info> {
    /// [Quarry].
    #[account(
        init,
        seeds = [
            b"Quarry".as_ref(),
            auth.rewarder.key().to_bytes().as_ref(),
            token_mint.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Quarry::LEN
    )]
    pub quarry: Account<'info, Quarry>,

    /// [Rewarder] authority.
    pub auth: MutableRewarderWithAuthority<'info>,

    /// [Mint] of the token to create a [Quarry] for.
    pub token_mint: Account<'info, Mint>,

    /// Payer of [Quarry] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Unused variable that held the clock. Placeholder.
    /// CHECK: OK
    pub unused_account: UncheckedAccount<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> Validate<'info> for CreateQuarry<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        invariant!(!self.auth.rewarder.is_paused, Paused);
        Ok(())
    }
}
