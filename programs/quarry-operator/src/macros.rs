//! Macros.

#[macro_export]
macro_rules! gen_operator_signer_seeds {
    ($operator:expr) => {
        &[
            b"Operator" as &[u8],
            &$operator.base.to_bytes(),
            &[$operator.bump],
        ]
    };
}
