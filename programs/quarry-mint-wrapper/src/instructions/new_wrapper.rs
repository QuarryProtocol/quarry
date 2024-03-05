use crate::*;

pub fn handler(ctx: Context<NewWrapper>, hard_cap: u64) -> Result<()> {
    let mint_wrapper = &mut ctx.accounts.mint_wrapper;
    mint_wrapper.base = ctx.accounts.base.key();
    mint_wrapper.bump = ctx.bumps.mint_wrapper;
    mint_wrapper.hard_cap = hard_cap;
    mint_wrapper.admin = ctx.accounts.admin.key();
    mint_wrapper.pending_admin = Pubkey::default();
    mint_wrapper.token_mint = ctx.accounts.token_mint.key();
    mint_wrapper.num_minters = 0;

    mint_wrapper.total_allowance = 0;
    mint_wrapper.total_minted = 0;

    emit!(NewMintWrapperEvent {
        mint_wrapper: mint_wrapper.key(),
        hard_cap,
        admin: ctx.accounts.admin.key(),
        token_mint: ctx.accounts.token_mint.key()
    });

    Ok(())
}

#[derive(Accounts)]
pub struct NewWrapper<'info> {
    /// Base account.
    pub base: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"MintWrapper".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + MintWrapper::LEN
    )]
    pub mint_wrapper: Account<'info, MintWrapper>,

    /// CHECK: Admin-to-be of the [MintWrapper].
    pub admin: UncheckedAccount<'info>,

    /// Token mint to mint.
    pub token_mint: Account<'info, Mint>,

    /// Token program.
    pub token_program: Program<'info, Token>,

    /// Payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> Validate<'info> for NewWrapper<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(self.token_mint.mint_authority.unwrap(), self.mint_wrapper);
        assert_keys_eq!(self.token_mint.freeze_authority.unwrap(), self.mint_wrapper);
        Ok(())
    }
}
