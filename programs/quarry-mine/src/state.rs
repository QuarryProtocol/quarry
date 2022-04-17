//! State structs.

use crate::*;

/// Controls token rewards distribution to all [Quarry]s.
/// The [Rewarder] is also the [quarry_mint_wrapper::Minter] registered to the [quarry_mint_wrapper::MintWrapper].
#[account]
#[derive(Copy, Default, Debug)]
pub struct Rewarder {
    /// Random pubkey used for generating the program address.
    pub base: Pubkey,
    /// Bump seed for program address.
    pub bump: u8,

    /// Authority who controls the rewarder
    pub authority: Pubkey,
    /// Pending authority which must accept the authority
    pub pending_authority: Pubkey,

    /// Number of [Quarry]s the [Rewarder] manages.
    /// If more than this many [Quarry]s are desired, one can create
    /// a second rewarder.
    pub num_quarries: u16,
    /// Amount of reward tokens distributed per day
    pub annual_rewards_rate: u64,
    /// Total amount of rewards shares allocated to [Quarry]s
    pub total_rewards_shares: u64,
    /// Mint wrapper.
    pub mint_wrapper: Pubkey,
    /// Mint of the rewards token for this [Rewarder].
    pub rewards_token_mint: Pubkey,

    /// Claim fees are placed in this account.
    pub claim_fee_token_account: Pubkey,
    /// Maximum amount of tokens to send to the Quarry DAO on each claim,
    /// in terms of milliBPS. 1,000 milliBPS = 1 BPS = 0.01%
    /// This is stored on the [Rewarder] to ensure that the fee will
    /// not exceed this in the future.
    pub max_claim_fee_millibps: u64,

    /// Authority allowed to pause a [Rewarder].
    pub pause_authority: Pubkey,
    /// If true, all instructions on the [Rewarder] are paused other than [quarry_mine::unpause].
    pub is_paused: bool,
}

impl Rewarder {
    pub const LEN: usize = 32 + 1 + 32 + 32 + 2 + 8 + 8 + 32 + 32 + 32 + 8 + 32 + 1;

    /// Asserts that this [Rewarder] is not paused.
    pub fn assert_not_paused(&self) -> Result<()> {
        invariant!(!self.is_paused, Paused);
        Ok(())
    }
}

/// A pool which distributes tokens to its [Miner]s.
#[account]
#[derive(Copy, Default)]
pub struct Quarry {
    /// Rewarder which manages this quarry
    pub rewarder: Pubkey,
    /// LP token this quarry is designated to
    pub token_mint_key: Pubkey,
    /// Bump.
    pub bump: u8,

    /// Index of the [Quarry].
    pub index: u16,
    /// Decimals on the token [Mint].
    pub token_mint_decimals: u8, // This field is never used.
    /// Timestamp when quarry rewards cease
    pub famine_ts: i64,
    /// Timestamp of last checkpoint
    pub last_update_ts: i64,
    /// Rewards per token stored in the quarry
    pub rewards_per_token_stored: u128,
    /// Amount of rewards distributed to the quarry per year.
    pub annual_rewards_rate: u64,
    /// Rewards shared allocated to this quarry
    pub rewards_share: u64,

    /// Total number of tokens deposited into the quarry.
    pub total_tokens_deposited: u64,
    /// Number of [Miner]s.
    pub num_miners: u64,
}

impl Quarry {
    pub const LEN: usize = 32 + 32 + 1 + 2 + 1 + 8 + 8 + 16 + 8 + 8 + 8 + 8;
}

/// An account that has staked tokens into a [Quarry].
#[account]
#[derive(Copy, Default, Debug)]
pub struct Miner {
    /// Key of the [Quarry] this [Miner] works on.
    pub quarry: Pubkey,
    /// Authority who manages this [Miner].
    /// All withdrawals of tokens must accrue to [TokenAccount]s owned by this account.
    pub authority: Pubkey,

    /// Bump.
    pub bump: u8,

    /// [TokenAccount] to hold the [Miner]'s staked LP tokens.
    pub token_vault_key: Pubkey,

    /// Stores the amount of tokens that the [Miner] may claim.
    /// Whenever the [Miner] claims tokens, this is reset to 0.
    pub rewards_earned: u64,

    /// A checkpoint of the [Quarry]'s reward tokens paid per staked token.
    ///
    /// When the [Miner] is initialized, this number starts at 0.
    /// On the first [quarry_mine::stake_tokens], the [Quarry]#update_rewards_and_miner
    /// method is called, which updates this checkpoint to the current quarry value.
    ///
    /// On a [quarry_mine::claim_rewards], the difference in checkpoints is used to calculate
    /// the amount of tokens owed.
    pub rewards_per_token_paid: u128,

    /// Number of tokens the [Miner] holds.
    pub balance: u64,

    /// Index of the [Miner].
    pub index: u64,
}

impl Miner {
    pub const LEN: usize = 32 + 32 + 1 + 32 + 8 + 16 + 8 + 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewarder_len() {
        assert_eq!(
            Rewarder::default().try_to_vec().unwrap().len(),
            Rewarder::LEN
        );
    }

    #[test]
    fn test_quarry_len() {
        assert_eq!(Quarry::default().try_to_vec().unwrap().len(), Quarry::LEN);
    }

    #[test]
    fn test_miner_len() {
        assert_eq!(Miner::default().try_to_vec().unwrap().len(), Miner::LEN);
    }
}
