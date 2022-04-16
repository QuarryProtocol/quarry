use crate::{utils::execute_ix_handler, *};

pub fn handler(ctx: Context<ClaimRewards>) -> Result<()> {
    execute_ix_handler(
        ctx.program_id,
        vec![
            ctx.accounts.mint_wrapper.to_account_info(),
            ctx.accounts.mint_wrapper_program.to_account_info(),
            ctx.accounts.minter.to_account_info(),
            ctx.accounts.rewards_token_mint.to_account_info(),
            ctx.accounts.rewards_token_account.to_account_info(),
            ctx.accounts.claim_fee_token_account.to_account_info(),
            ctx.accounts.claim.authority.to_account_info(),
            ctx.accounts.claim.miner.to_account_info(),
            ctx.accounts.claim.quarry.to_account_info(),
            ctx.accounts.claim.token_program.to_account_info(),
            ctx.accounts.claim.rewarder.to_account_info(),
        ],
        instructions::claim_rewards_v2::handler,
    )
}

/// ClaimRewards accounts
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    /// Mint wrapper.
    #[account(mut)]
    pub mint_wrapper: Box<Account<'info, quarry_mint_wrapper::MintWrapper>>,
    /// Mint wrapper program.
    pub mint_wrapper_program: Program<'info, quarry_mint_wrapper::program::QuarryMintWrapper>,
    /// [quarry_mint_wrapper::Minter] information.
    #[account(mut)]
    pub minter: Box<Account<'info, quarry_mint_wrapper::Minter>>,

    /// Mint of the rewards token.
    #[account(mut)]
    pub rewards_token_mint: Box<Account<'info, Mint>>,

    /// Account to claim rewards for.
    #[account(mut)]
    pub rewards_token_account: Box<Account<'info, TokenAccount>>,

    /// Account to send claim fees to.
    #[account(mut)]
    pub claim_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// Claim accounts
    pub claim: UserClaim<'info>,
}

/// Claim accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
#[derive(Accounts, Clone)]
pub struct UserClaim<'info> {
    /// Miner authority (i.e. the user).
    pub authority: Signer<'info>,

    /// Miner.
    #[account(mut)]
    pub miner: Account<'info, Miner>,

    /// Quarry to claim from.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,

    /// Placeholder for the miner vault.
    /// CHECK: OK
    pub unused_miner_vault: UncheckedAccount<'info>,

    /// Placeholder for the user's staked token account.
    /// CHECK: OK
    pub unused_token_account: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rewarder
    pub rewarder: Account<'info, Rewarder>,
}

impl<'info> Validate<'info> for ClaimRewards<'info> {
    /// Validates a [ClaimRewards] accounts struct.
    fn validate(&self) -> Result<()> {
        self.claim.validate()?;
        self.claim.rewarder.assert_not_paused()?;

        assert_keys_eq!(self.mint_wrapper, self.claim.rewarder.mint_wrapper);
        assert_keys_eq!(self.mint_wrapper.token_mint, self.rewards_token_mint);

        assert_keys_eq!(self.minter.mint_wrapper, self.mint_wrapper);
        assert_keys_eq!(self.minter.minter_authority, self.claim.rewarder);

        // rewards_token_mint validate
        assert_keys_eq!(
            self.rewards_token_mint,
            self.claim.rewarder.rewards_token_mint
        );
        assert_keys_eq!(
            self.rewards_token_mint.mint_authority.unwrap(),
            self.mint_wrapper
        );

        // rewards_token_account validate
        assert_keys_eq!(self.rewards_token_account.mint, self.rewards_token_mint);

        // claim_fee_token_account validate
        assert_keys_eq!(
            self.claim_fee_token_account,
            self.claim.rewarder.claim_fee_token_account
        );
        assert_keys_eq!(self.claim_fee_token_account.mint, self.rewards_token_mint);

        Ok(())
    }
}

impl<'info> Validate<'info> for UserClaim<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority);

        // quarry
        assert_keys_eq!(self.miner.quarry, self.quarry);

        // rewarder
        assert_keys_eq!(self.quarry.rewarder, self.rewarder);

        Ok(())
    }
}
