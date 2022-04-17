use crate::{utils::execute_ix_handler, *};

pub fn handler(ctx: Context<CreateQuarry>) -> Result<()> {
    execute_ix_handler(
        ctx.program_id,
        vec![
            ctx.accounts.quarry.to_account_info(),
            ctx.accounts.auth.authority.to_account_info(),
            ctx.accounts.auth.rewarder.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        crate::quarry_mine::create_quarry_v2,
    )
}

/// Accounts for [quarry_mine::create_quarry].
#[derive(Accounts)]
pub struct CreateQuarry<'info> {
    /// [Quarry].
    #[account(mut)]
    pub quarry: SystemAccount<'info>,

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
