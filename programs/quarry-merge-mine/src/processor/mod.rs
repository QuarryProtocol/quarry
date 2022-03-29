//! Processes incoming instructions.
#![deny(clippy::integer_arithmetic, clippy::float_arithmetic)]

pub(crate) mod claim;
pub(crate) mod deposit;
pub(crate) mod init;
pub mod rescue_tokens;
pub(crate) mod withdraw;

pub use rescue_tokens::*;
