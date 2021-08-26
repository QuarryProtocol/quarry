//! Macros.

#[macro_export]
macro_rules! gen_wrapper_signer_seeds {
    ($wrapper:expr) => {
        &[
            b"MintWrapper" as &[u8],
            &$wrapper.base.to_bytes(),
            &[$wrapper.bump],
        ]
    };
}
