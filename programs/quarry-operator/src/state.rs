//! State structs.

pub use crate::*;

/// Operator state
#[account]
#[derive(Copy, Default, Debug, PartialEq, Eq)]
pub struct Operator {
    /// The base.
    pub base: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// The [Rewarder].
    pub rewarder: Pubkey,
    /// Can modify the authorities below.
    pub admin: Pubkey,

    /// Can call [quarry_mine::quarry_mine::set_annual_rewards].
    pub rate_setter: Pubkey,
    /// Can call [quarry_mine::quarry_mine::create_quarry].
    pub quarry_creator: Pubkey,
    /// Can call [quarry_mine::quarry_mine::set_rewards_share].
    pub share_allocator: Pubkey,

    /// When the [Operator] was last modified.
    pub last_modified_ts: i64,
    /// Auto-incrementing sequence number of the set of authorities.
    /// Useful for checking if things were updated.
    pub generation: u64,
}

impl Operator {
    /// Number of bytes in an [Operator].
    pub const LEN: usize = 32 + 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8;

    pub(crate) fn record_update(&mut self) -> Result<()> {
        self.last_modified_ts = Clock::get()?.unix_timestamp;
        self.generation = unwrap_int!(self.generation.checked_add(1));
        Ok(())
    }
}
