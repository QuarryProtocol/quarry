use crate::*;

pub fn handler(ctx: Context<ClaimRewardsV2>) -> Result<()> {
    let miner = &mut ctx.accounts.claim.miner;

    let now = Clock::get()?.unix_timestamp;
    let quarry = &mut ctx.accounts.claim.quarry;
    quarry.update_rewards_and_miner(miner, &ctx.accounts.claim.rewarder, now)?;

    ctx.accounts.calculate_and_claim_rewards()?;

    Ok(())
}

impl<'info> ClaimRewardsV2<'info> {
    /// Calculates rewards and claims them.
    pub fn calculate_and_claim_rewards(&mut self) -> Result<()> {
        let miner = &mut self.claim.miner;
        let amount_claimable = miner.rewards_earned;
        if amount_claimable == 0 {
            // 0 claimable -- skip all logic
            return Ok(());
        }

        // Calculate rewards
        let max_claim_fee_millibps = self.claim.rewarder.max_claim_fee_millibps;
        invariant!(
            max_claim_fee_millibps < MAX_BPS * DEFAULT_CLAIM_FEE_MILLIBPS,
            InvalidMaxClaimFee
        );
        let max_claim_fee = unwrap_int!(::u128::mul_div_u64(
            amount_claimable,
            max_claim_fee_millibps,
            MAX_BPS * DEFAULT_CLAIM_FEE_MILLIBPS
        ));

        let amount_claimable_minus_fees = unwrap_int!(amount_claimable.checked_sub(max_claim_fee));

        // Claim all rewards.
        miner.rewards_earned = 0;

        // Setup remaining variables
        self.mint_claimed_tokens(amount_claimable_minus_fees)?;
        self.mint_fees(max_claim_fee)?;

        let now = Clock::get()?.unix_timestamp;
        emit!(ClaimEvent {
            authority: self.claim.authority.key(),
            staked_token: self.claim.quarry.token_mint_key,
            timestamp: now,
            rewards_token: self.rewards_token_mint.key(),
            amount: amount_claimable_minus_fees,
            fees: max_claim_fee,
        });

        Ok(())
    }

    /// Mints the claimed tokens.
    fn mint_claimed_tokens(&self, amount_claimable_minus_fees: u64) -> Result<()> {
        let rewards_token_account = (*self.rewards_token_account).clone();
        self.perform_mint(rewards_token_account, amount_claimable_minus_fees)
    }

    /// Mints the fee tokens.
    fn mint_fees(&self, claim_fee: u64) -> Result<()> {
        let claim_fee_token_account = (*self.claim_fee_token_account).clone();
        self.perform_mint(claim_fee_token_account, claim_fee)
    }

    fn create_perform_mint_accounts(
        &self,
        destination: Account<'info, TokenAccount>,
    ) -> quarry_mint_wrapper::cpi::accounts::PerformMint<'info> {
        quarry_mint_wrapper::cpi::accounts::PerformMint {
            mint_wrapper: self.mint_wrapper.to_account_info(),
            minter_authority: self.claim.rewarder.to_account_info(),
            token_mint: self.rewards_token_mint.to_account_info(),
            destination: destination.to_account_info(),
            minter: self.minter.to_account_info(),
            token_program: self.claim.token_program.to_account_info(),
        }
    }

    fn perform_mint(&self, destination: Account<'info, TokenAccount>, amount: u64) -> Result<()> {
        let claim_mint_accounts = self.create_perform_mint_accounts(destination);

        // Create the signer seeds.
        let seeds = gen_rewarder_signer_seeds!(self.claim.rewarder);
        let signer_seeds = &[&seeds[..]];

        quarry_mint_wrapper::cpi::perform_mint(
            CpiContext::new_with_signer(
                self.mint_wrapper_program.to_account_info(),
                claim_mint_accounts,
                signer_seeds,
            ),
            amount,
        )
    }
}

/// ClaimRewardsV2 accounts
#[derive(Accounts)]
pub struct ClaimRewardsV2<'info> {
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
    pub rewards_token_mint: Account<'info, Mint>,

    /// Account to claim rewards for.
    #[account(mut)]
    pub rewards_token_account: Box<Account<'info, TokenAccount>>,

    /// Account to send claim fees to.
    #[account(mut)]
    pub claim_fee_token_account: Box<Account<'info, TokenAccount>>,

    /// Claim accounts
    pub claim: UserClaimV2<'info>,
}

/// Claim accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
#[derive(Accounts, Clone)]
pub struct UserClaimV2<'info> {
    /// Miner authority (i.e. the user).
    pub authority: Signer<'info>,

    /// Miner.
    #[account(mut)]
    pub miner: Account<'info, Miner>,

    /// Quarry to claim from.
    #[account(mut)]
    pub quarry: Account<'info, Quarry>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rewarder
    pub rewarder: Account<'info, Rewarder>,
}

impl<'info> Validate<'info> for ClaimRewardsV2<'info> {
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

impl<'info> Validate<'info> for UserClaimV2<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(!self.rewarder.is_paused, Paused);
        // authority
        invariant!(self.authority.is_signer, Unauthorized);
        assert_keys_eq!(self.authority, self.miner.authority);

        // quarry
        assert_keys_eq!(self.miner.quarry_key, self.quarry);

        // rewarder
        assert_keys_eq!(self.quarry.rewarder_key, self.rewarder);

        Ok(())
    }
}
