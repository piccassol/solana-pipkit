//! # solana-pipkit
//!
//! Pragmatic Rust utilities for Solana program and client development.
//!
//! ## Features
//!
//! - **Account Utilities**: Validation, parsing, and batch account operations
//! - **PDA Helpers**: Easy PDA derivation with builder pattern
//! - **Token Operations**: SPL token transfers, burns, and account management
//! - **Rent Recovery**: Scan and close empty accounts to reclaim SOL
//! - **Transaction Batching**: Build and execute transactions efficiently
//! - **Account Graph**: Traverse and analyze account relationships
//! - **Anchor Helpers**: CPI builders, discriminators, and validation (optional)
//! - **Jupiter Integration**: DEX aggregator for token swaps (optional)
//! - **Safety Protocol**: Client-side safety checks to prevent common mistakes
//!
//! ## Feature Flags
//!
//! - `anchor` - Enable Anchor framework helpers and CPI utilities
//! - `jupiter` - Enable Jupiter DEX integration for token swaps
//! - `all` - Enable all optional features
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use solana_pipkit::prelude::*;
//!
//! // Derive a PDA
//! let (pda, bump) = PdaBuilder::new()
//!     .add_string("user")
//!     .add_pubkey(&wallet)
//!     .derive(&program_id);
//!
//! // Build a transaction with batching
//! let builder = TransactionBuilder::new()
//!     .add_instruction(transfer_ix)
//!     .compute_units(200_000)
//!     .priority_fee(1000);
//!
//! // Verify address before sending
//! let recipient = AddressVerifier::verify_address("7xKX...8AsU")?;
//!
//! // Validate amount
//! let validation = AmountValidator::validate_amount(amount, 9, balance);
//! ```

pub mod account_graph;
pub mod account_utils;
pub mod anchor_helpers;
pub mod error;
pub mod pda;
pub mod rent_cleaner;
pub mod safety;
pub mod token_utils;
pub mod transaction;

#[cfg(feature = "jupiter")]
pub mod jupiter;

pub use error::{Result, ToolkitError};

/// Common imports for convenient use.
///
/// Import everything you need with:
/// ```rust,ignore
/// use solana_pipkit::prelude::*;
/// ```
pub mod prelude {
    // Core utilities
    pub use crate::account_utils::*;
    pub use crate::pda::*;
    pub use crate::token_utils::*;
    pub use crate::{Result, ToolkitError};

    // Rent recovery
    pub use crate::rent_cleaner::{
        AccountType, AdvancedCleanupConfig, AdvancedRentCleaner, CleanableAccount,
        CleanupPriority, CleanupResult, CleanupStrategy, RentCleaner, RentCleanerConfig,
    };

    // Transaction utilities
    pub use crate::transaction::{
        BatchExecutor, BatchResult, ParallelBatchExecutor, TransactionBuilder,
        TransactionConfig,
    };

    // Account graph
    pub use crate::account_graph::{
        AccountEdge, AccountGraph, AccountGraphBuilder, AccountNode, AccountNodeType,
        EdgeType,
    };

    // Anchor helpers
    pub use crate::anchor_helpers::{
        account_discriminator, instruction_discriminator, programs, CpiInstructionBuilder,
        RemainingAccountsBuilder,
    };

    // Safety protocol
    pub use crate::safety::{
        AddressComparison, AddressVerification, AddressVerifier,
        AmountValidation, AmountValidator, AmountWarning, MagnitudeCheck,
        RiskLevel, SafetyProtocol, SafetyReport, WarningSeverity,
        LAMPORTS_PER_SOL,
    };

    #[cfg(feature = "jupiter")]
    pub use crate::jupiter::*;
}
