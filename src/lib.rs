//! # solana-pipkit
//!
//! Pragmatic Rust utilities for Solana program and client development.

pub mod account_utils;
pub mod error;
pub mod pda;
pub mod rent_cleaner;
pub mod token_utils;

#[cfg(feature = "jupiter")]
pub mod jupiter;

pub use error::{Result, ToolkitError};

/// Common imports
pub mod prelude {
    pub use crate::account_utils::*;
    pub use crate::pda::*;
    pub use crate::rent_cleaner::*;
    pub use crate::token_utils::*;
    pub use crate::{Result, ToolkitError};

    #[cfg(feature = "jupiter")]
    pub use crate::jupiter::*;
}
