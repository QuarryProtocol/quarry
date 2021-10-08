//! Macros for [crate::quarry_merge_mine].

macro_rules! gen_pool_signer_seeds {
    ($pool:expr) => {
        &[
            b"MergePool" as &[u8],
            &$pool.primary_mint.to_bytes(),
            &[$pool.bump],
        ]
    };
}

macro_rules! gen_merge_miner_signer_seeds {
    ($miner:expr) => {
        &[
            b"MergeMiner" as &[u8],
            &$miner.pool.to_bytes(),
            &$miner.owner.to_bytes(),
            &[$miner.bump],
        ]
    };
}
