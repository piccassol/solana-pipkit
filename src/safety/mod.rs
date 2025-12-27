//! Safety protocol for Solana transactions.
//!
//! This module provides client-side safety checks to prevent common mistakes
//! that cause crypto losses: typo addresses, fake tokens, rug pulls, and more.

pub mod address_verify;
pub mod amount_validation;
pub mod validator;

pub use address_verify::*;
pub use amount_validation::*;
pub use validator::*;
