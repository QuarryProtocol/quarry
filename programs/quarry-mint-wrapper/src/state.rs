//! State structs.

use crate::*;

/// Mint wrapper
///
/// ```ignore
/// seeds = [
///     b"MintWrapper",
///     base.key().to_bytes().as_ref(),
///     &[bump]
/// ],
///
#[account]
#[derive(Copy, Default, Debug)]
pub struct MintWrapper {
    /// Base account.
    pub base: Pubkey,
    /// Bump for allowing the proxy mint authority to sign.
    pub bump: u8,
    /// Maximum number of tokens that can be issued.
    pub hard_cap: u64,

    /// Admin account.
    pub admin: Pubkey,
    /// Next admin account.
    pub pending_admin: Pubkey,

    /// Mint of the token.
    pub token_mint: Pubkey,
    /// Number of [Minter]s.
    pub num_minters: u64,

    /// Total allowance outstanding.
    pub total_allowance: u64,
    /// Total amount of tokens minted through the [MintWrapper].
    pub total_minted: u64,
}

impl MintWrapper {
    /// Number of bytes that a [MintWrapper] struct takes up.
    pub const LEN: usize = 32 + 1 + 8 + 32 + 32 + 32 + 8 + 8 + 8;
}

/// One who can mint.
///
/// ```ignore
/// seeds = [
///     b"MintWrapperMinter",
///     auth.mint_wrapper.key().to_bytes().as_ref(),
///     minter_authority.key().to_bytes().as_ref(),
///     &[bump]
/// ],
/// ```
#[account]
#[derive(Copy, Default, Debug)]
pub struct Minter {
    /// The mint wrapper.
    pub mint_wrapper: Pubkey,
    /// Address that can mint.
    pub minter_authority: Pubkey,
    /// Bump seed.
    pub bump: u8,

    /// Auto-incrementing index of the [Minter].
    pub index: u64,

    /// Limit of number of tokens that this [Minter] can mint.
    pub allowance: u64,
    /// Cumulative sum of the number of tokens ever minted by this [Minter].
    pub total_minted: u64,
}

impl Minter {
    /// Number of bytes that a [Minter] struct takes up.
    pub const LEN: usize = 32 + 32 + 1 + 8 + 8 + 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_wrapper_len() {
        assert_eq!(
            MintWrapper::default().try_to_vec().unwrap().len(),
            MintWrapper::LEN
        );
    }

    #[test]
    fn test_minter_len() {
        assert_eq!(Minter::default().try_to_vec().unwrap().len(), Minter::LEN);
    }
}
