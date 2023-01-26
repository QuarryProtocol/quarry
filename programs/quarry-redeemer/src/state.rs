//! State structs.

use crate::*;

/// Redeemer state
#[account]
#[derive(Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Redeemer {
    /// [Mint] of the IOU token.
    pub iou_mint: Pubkey,
    /// [Mint] of the token to redeem.
    pub redemption_mint: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// Lifetime number of IOU tokens redeemed for redemption tokens.
    pub total_tokens_redeemed: u64,
}

impl Redeemer {
    /// Number of bytes in a [Redeemer].
    pub const LEN: usize = 32 + 32 + 1 + 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redeemer_len() {
        assert_eq!(
            Redeemer::default().try_to_vec().unwrap().len(),
            Redeemer::LEN
        );
    }
}
