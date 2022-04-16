use quarry_mine::execute_ix_handler;

use crate::*;

/// Calls [quarry_mine::quarry_mine::create_quarry].
pub fn handler(ctx: Context<DelegateCreateQuarry>) -> Result<()> {
    execute_ix_handler(
        ctx.program_id,
        DelegateCreateQuarryV2 {
            with_delegate: ctx.accounts.with_delegate.clone(),
            quarry: ctx.accounts.quarry.clone(),
            token_mint: ctx.accounts.token_mint.clone(),
            payer: ctx.accounts.payer.clone(),
            system_program: ctx.accounts.system_program.clone(),
        },
        instructions::delegate_create_quarry_v2::handler,
    )
}

/// Accounts for [crate::quarry_operator::delegate_create_quarry].
#[derive(Accounts)]
pub struct DelegateCreateQuarry<'info> {
    /// Delegation accounts.
    pub with_delegate: WithDelegate<'info>,

    /// The Quarry to create.
    #[account(mut)]
    pub quarry: SystemAccount<'info>,

    /// Mint of the Quarry being created.
    pub token_mint: Box<Account<'info, anchor_spl::token::Mint>>,

    /// Payer of [Quarry] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Unused variable that held the clock. Placeholder.
    /// CHECK: OK
    pub unused_clock: UncheckedAccount<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> Validate<'info> for DelegateCreateQuarry<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.with_delegate.operator.quarry_creator,
            self.with_delegate.delegate,
            Unauthorized
        );
        self.with_delegate.validate()?;
        Ok(())
    }
}
