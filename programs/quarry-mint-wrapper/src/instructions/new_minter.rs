use crate::*;

pub fn handler(ctx: Context<NewMinter>) -> Result<()> {
    let minter = &mut ctx.accounts.minter;

    minter.mint_wrapper = ctx.accounts.auth.mint_wrapper.key();
    minter.minter_authority = ctx.accounts.new_minter_authority.key();
    minter.bump = ctx.bumps.minter;

    let index = ctx.accounts.auth.mint_wrapper.num_minters;
    minter.index = index;

    // update num minters
    let mint_wrapper = &mut ctx.accounts.auth.mint_wrapper;
    mint_wrapper.num_minters = unwrap_int!(index.checked_add(1));

    minter.allowance = 0;
    minter.total_minted = 0;

    emit!(NewMinterEvent {
        mint_wrapper: minter.mint_wrapper,
        minter: minter.key(),
        index: minter.index,
        minter_authority: minter.minter_authority,
    });
    Ok(())
}

/// Adds a minter.
#[derive(Accounts)]
pub struct NewMinter<'info> {
    /// Owner of the [MintWrapper].
    pub auth: OnlyAdmin<'info>,

    /// Account to authorize as a minter.
    /// CHECK: Can be any Solana account.
    pub new_minter_authority: UncheckedAccount<'info>,

    /// Information about the minter.
    #[account(
        init,
        seeds = [
            b"MintWrapperMinter".as_ref(),
            auth.mint_wrapper.key().to_bytes().as_ref(),
            new_minter_authority.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Minter::LEN
    )]
    pub minter: Account<'info, Minter>,

    /// Payer for creating the minter.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

impl<'info> Validate<'info> for NewMinter<'info> {
    fn validate(&self) -> Result<()> {
        self.auth.validate()?;
        Ok(())
    }
}
