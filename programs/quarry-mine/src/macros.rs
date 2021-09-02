//! Macros.

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
