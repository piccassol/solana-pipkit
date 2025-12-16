//! Error types for the toolkit.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolkitError {
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    #[error("Insufficient balance: need {needed} lamports, have {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    #[error("Invalid PDA: {0}")]
    InvalidPda(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("Signature error: {0}")]
    SignatureError(#[from] solana_sdk::signature::SignerError),

    #[error("Borsh serialization error: {0}")]
    BorshError(#[from] std::io::Error),

    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<&str> for ToolkitError {
    fn from(s: &str) -> Self {
        ToolkitError::Custom(s.to_string())
    }
}

impl From<String> for ToolkitError {
    fn from(s: String) -> Self {
        ToolkitError::Custom(s)
    }
}
