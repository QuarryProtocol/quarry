//! Macros.

#[macro_export]
macro_rules! gen_redeemer_signer_seeds {
    ($redeemer:expr) => {
        &[
            b"Redeemer" as &[u8],
            &$redeemer.iou_mint.to_bytes(),
            &$redeemer.redemption_mint.to_bytes(),
            &[$redeemer.bump],
        ]
    };
}
