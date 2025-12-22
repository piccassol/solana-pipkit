//! Error types for solana-pipkit.
//!
//! This module provides a comprehensive error type for all toolkit operations.

use thiserror::Error;

/// Main error type for solana-pipkit operations.
#[derive(Error, Debug)]
pub enum ToolkitError {
    /// RPC client error from Solana.
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    /// Transaction execution or building error.
    #[error("Transaction error: {0}")]
    TransactionError(String),

    /// Account not found on chain.
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Invalid or unexpected account data.
    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    /// Failed to deserialize account data.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Insufficient lamports for operation.
    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance {
        /// Lamports needed.
        needed: u64,
        /// Lamports available.
        available: u64,
    },

    /// Invalid PDA derivation.
    #[error("Invalid PDA: {0}")]
    InvalidPda(String),

    /// SPL Token program error.
    #[error("Token error: {0}")]
    TokenError(String),

    /// Signature or signing error.
    #[error("Signature error: {0}")]
    SignatureError(#[from] solana_sdk::signature::SignerError),

    /// Network or connection error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Failed to parse data.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Jupiter DEX aggregator error.
    #[error("Jupiter error: {0}")]
    JupiterError(String),

    /// Error during transaction signing.
    #[error("Signing error: {0}")]
    SigningError(String),

    /// Batch operation error with details.
    #[error("Batch error: {successful} succeeded, {failed} failed")]
    BatchError {
        /// Number of successful operations.
        successful: usize,
        /// Number of failed operations.
        failed: usize,
        /// Error messages from failures.
        errors: Vec<String>,
    },

    /// Graph traversal error.
    #[error("Graph error: {0}")]
    GraphError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Operation timeout.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Custom error with message.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl ToolkitError {
    /// Create a transaction error from a string.
    pub fn transaction<S: Into<String>>(msg: S) -> Self {
        Self::TransactionError(msg.into())
    }

    /// Create an account not found error.
    pub fn account_not_found<S: Into<String>>(pubkey: S) -> Self {
        Self::AccountNotFound(pubkey.into())
    }

    /// Create an invalid account data error.
    pub fn invalid_data<S: Into<String>>(msg: S) -> Self {
        Self::InvalidAccountData(msg.into())
    }

    /// Create a custom error.
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Self::Custom(msg.into())
    }

    /// Check if this is a retryable error (network issues, etc.).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RpcError(_) | Self::NetworkError(_) | Self::Timeout(_)
        )
    }
}

/// Result type alias for toolkit operations.
pub type Result<T> = std::result::Result<T, ToolkitError>;
