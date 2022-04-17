//! Macros.

/// Generates the signer seeds for a [crate::Rewarder].
#[macro_export]
macro_rules! gen_rewarder_signer_seeds {
    ($rewarder:expr) => {
        &[
            b"Rewarder".as_ref(),
            $rewarder.base.as_ref(),
            &[$rewarder.bump],
        ]
    };
}

/// Generates the signer seeds for a [crate::Miner].
#[macro_export]
macro_rules! gen_miner_signer_seeds {
    ($miner:expr) => {
        &[
            b"Miner".as_ref(),
            $miner.quarry.as_ref(),
            $miner.authority.as_ref(),
            &[$miner.bump],
        ]
    };
}
