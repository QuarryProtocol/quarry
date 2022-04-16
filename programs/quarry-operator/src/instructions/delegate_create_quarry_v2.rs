use crate::*;

/// Calls [quarry_mine::quarry_mine::create_quarry].
pub fn handler(ctx: Context<DelegateCreateQuarryV2>) -> Result<()> {
    let operator = &ctx.accounts.with_delegate.operator;
    let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
    quarry_mine::cpi::create_quarry_v2(CpiContext::new_with_signer(
        ctx.accounts
            .with_delegate
            .quarry_mine_program
            .to_account_info(),
        quarry_mine::cpi::accounts::CreateQuarryV2 {
            quarry: ctx.accounts.quarry.to_account_info(),
            auth: ctx.accounts.with_delegate.to_auth_accounts(),
            token_mint: ctx.accounts.token_mint.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
        signer_seeds,
    ))?;
    Ok(())
}

/// Accounts for [crate::quarry_operator::delegate_create_quarry_v2].
#[derive(Accounts)]
pub struct DelegateCreateQuarryV2<'info> {
    pub with_delegate: WithDelegate<'info>,

    /// The Quarry to create.
    #[account(mut)]
    pub quarry: SystemAccount<'info>,

    /// Mint of the Quarry being created.
    pub token_mint: Box<Account<'info, anchor_spl::token::Mint>>,

    /// Payer of [Quarry] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> Validate<'info> for DelegateCreateQuarryV2<'info> {
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
