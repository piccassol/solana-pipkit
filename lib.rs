//! # solana-pipkit
//!
//! Pragmatic Rust utilities for Solana program and client development.
//!
//! ## Modules
//!
//! - **Core**: Account utils, PDA helpers, rent cleaning, token utilities
//! - **Safety**: Transaction validation, address verification, token safety, MEV protection
//! - **Speed**: Multi-RPC pooling, blockhash caching, sniper module, Jito bundles
//! - **Analytics**: Wallet profiling, holder analysis, pattern detection, PnL tracking
//! - **DeFi**: Pool interactions, liquidity provision, yield farming, LP token parsing
//!
//! ## Feature Flags
//!
//! - `jupiter` - Jupiter aggregator integration
//! - `jito` - Jito bundle support for MEV protection
//! - `anchor` - Anchor framework utilities
//! - `analytics` - Wallet and token analytics
//! - `defi` - DeFi protocol integrations
//! - `safety` - Transaction safety validation
//! - `speed` - Performance optimizations
//! - `all` - Enable all features

pub mod account_utils;
pub mod error;
pub mod pda;
pub mod rent_cleaner;
pub mod token_utils;

#[cfg(feature = "jupiter")]
pub mod jupiter;

#[cfg(feature = "safety")]
pub mod safety;

#[cfg(feature = "speed")]
pub mod speed;

#[cfg(feature = "analytics")]
pub mod analytics;

#[cfg(feature = "defi")]
pub mod defi;

pub use error::{Result, ToolkitError};

/// Common imports for quick access
pub mod prelude {
    pub use crate::account_utils::*;
    pub use crate::pda::*;
    pub use crate::rent_cleaner::*;
    pub use crate::token_utils::*;
    pub use crate::{Result, ToolkitError};

    #[cfg(feature = "jupiter")]
    pub use crate::jupiter::*;

    #[cfg(feature = "safety")]
    pub use crate::safety::*;

    #[cfg(feature = "speed")]
    pub use crate::speed::*;

    #[cfg(feature = "analytics")]
    pub use crate::analytics::*;

    #[cfg(feature = "defi")]
    pub use crate::defi::*;
}
