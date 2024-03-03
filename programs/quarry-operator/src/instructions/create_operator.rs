use crate::*;

pub fn handler(ctx: Context<CreateOperator>) -> Result<()> {
    let operator = &mut ctx.accounts.operator;
    operator.base = ctx.accounts.base.key();
    operator.bump = ctx.bumps.operator;

    operator.rewarder = ctx.accounts.rewarder.key();
    operator.admin = ctx.accounts.admin.key();

    operator.rate_setter = operator.admin;
    operator.quarry_creator = operator.admin;
    operator.share_allocator = operator.admin;
    operator.record_update()?;

    let signer_seeds: &[&[&[u8]]] = &[gen_operator_signer_seeds!(operator)];
    quarry_mine::cpi::accept_authority(CpiContext::new_with_signer(
        ctx.accounts.quarry_mine_program.to_account_info(),
        quarry_mine::cpi::accounts::AcceptAuthority {
            authority: ctx.accounts.operator.to_account_info(),
            rewarder: ctx.accounts.rewarder.to_account_info(),
        },
        signer_seeds,
    ))?;

    Ok(())
}

/// Accounts for [crate::quarry_operator::create_operator].
#[derive(Accounts)]
pub struct CreateOperator<'info> {
    /// Base key used to create the [Operator].
    pub base: Signer<'info>,
    /// Operator PDA.
    #[account(
        init,
        seeds = [
            b"Operator".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Operator::LEN
    )]
    pub operator: Account<'info, Operator>,
    /// [Rewarder] of the token.
    #[account(mut)]
    pub rewarder: Box<Account<'info, Rewarder>>,
    /// CHECK: The admin to set.
    pub admin: UncheckedAccount<'info>,
    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,
    /// [System] program.
    pub system_program: Program<'info, System>,
    /// Quarry mine
    pub quarry_mine_program: Program<'info, quarry_mine::program::QuarryMine>,
}

impl<'info> Validate<'info> for CreateOperator<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.operator,
            self.rewarder.pending_authority,
            PendingAuthorityNotSet
        );
        Ok(())
    }
}
