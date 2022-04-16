use crate::*;

/// Creates a [Miner] for the given authority.
///
/// Anyone can call this; this is an associated account.
pub fn handler(ctx: Context<CreateMiner>) -> Result<()> {
    let quarry = &mut ctx.accounts.quarry;
    let index = quarry.num_miners;
    quarry.num_miners = unwrap_int!(quarry.num_miners.checked_add(1));

    let miner = &mut ctx.accounts.miner;
    miner.authority = ctx.accounts.authority.key();
    miner.bump = unwrap_bump!(ctx, "miner");
    miner.quarry = ctx.accounts.quarry.key();
    miner.token_vault_key = ctx.accounts.miner_vault.key();
    miner.rewards_earned = 0;
    miner.rewards_per_token_paid = 0;
    miner.balance = 0;
    miner.index = index;

    emit!(MinerCreateEvent {
        authority: miner.authority,
        quarry: miner.quarry,
        miner: miner.key(),
    });

    Ok(())
}

/// Accounts for [quarry_mine::create_miner].
#[derive(Accounts)]
pub struct CreateMiner<'info> {
    /// Authority of the [Miner].
    pub authority: Signer<'info>,

    /// [Miner] to be created.
    #[account(
        init,
        seeds = [
            b"Miner".as_ref(),
            quarry.key().to_bytes().as_ref(),
            authority.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Miner::LEN
    )]
    pub miner: Box<Account<'info, Miner>>,

    /// [Quarry] to create a [Miner] for.
    #[account(mut)]
    pub quarry: Box<Account<'info, Quarry>>,

    /// [Rewarder].
    pub rewarder: Box<Account<'info, Rewarder>>,

    /// System program.
    pub system_program: Program<'info, System>,

    /// Payer of [Miner] creation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// [Mint] of the token to create a [Quarry] for.
    pub token_mint: Account<'info, Mint>,

    /// [TokenAccount] holding the token [Mint].
    pub miner_vault: Account<'info, TokenAccount>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

impl<'info> Validate<'info> for CreateMiner<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        assert_keys_eq!(self.miner_vault.owner, self.miner);
        assert_keys_eq!(self.miner_vault.mint, self.token_mint);
        invariant!(self.miner_vault.delegate.is_none());
        invariant!(self.miner_vault.close_authority.is_none());

        assert_keys_eq!(self.miner_vault.mint, self.quarry.token_mint_key);
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        Ok(())
    }
}

/// Triggered when a new miner is created.
#[event]
pub struct MinerCreateEvent {
    /// Authority of the miner.
    #[index]
    pub authority: Pubkey,
    /// Quarry the miner was created on.
    #[index]
    pub quarry: Pubkey,
    /// The [Miner].
    pub miner: Pubkey,
}
