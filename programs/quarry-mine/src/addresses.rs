//! Contains addresses used for the Quarry program.
//! These addresses are updated via program upgrades.

use anchor_lang::prelude::*;

/// Wrapper module.
pub mod fee_to {
    solana_program::declare_id!("4MMZH3ih1aSty2nx4MC3kSR94Zb55XsXnqb5jfEcyHWQ");
}

/// Wrapper module.
pub mod fee_setter {
    solana_program::declare_id!("4MMZH3ih1aSty2nx4MC3kSR94Zb55XsXnqb5jfEcyHWQ");
}

/// Account authorized to take fees.
pub static FEE_TO: Pubkey = fee_to::ID;

/// Account authorized to set fees of a rewarder. Currently unused.
pub static FEE_SETTER: Pubkey = fee_setter::ID;
