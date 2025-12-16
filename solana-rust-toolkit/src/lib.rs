//! # Solana Rust Toolkit
//!
//! A collection of utilities for Solana development in Rust.
//!
//! ## Modules
//!
//! - `rent_cleaner` - Reclaim SOL from empty accounts
//! - `token_utils` - SPL token operations
//! - `pda` - PDA derivation helpers
//! - `account_utils` - Account validation and parsing
//!
//! ## Example
//!
//! ```rust,ignore
//! use solana_rust_toolkit::prelude::*;
//!
//! let cleaner = RentCleaner::new(rpc_url, keypair);
//! let recovered = cleaner.clean_empty_accounts().await?;
//! ```

pub mod account_utils;
pub mod error;
pub mod pda;
pub mod rent_cleaner;
pub mod token_utils;

/// Common imports for convenience
pub mod prelude {
    pub use crate::account_utils::*;
    pub use crate::error::ToolkitError;
    pub use crate::pda::*;
    pub use crate::rent_cleaner::RentCleaner;
    pub use crate::token_utils::*;
}

pub use error::ToolkitError;
pub type Result<T> = std::result::Result<T, ToolkitError>;
