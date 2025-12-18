//! Error types for solana-pipkit.
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

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: u64, available: u64 },

    #[error("Invalid PDA: {0}")]
    InvalidPda(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("Signature error: {0}")]
    SignatureError(#[from] solana_sdk::signature::SignerError),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Jupiter error: {0}")]
    JupiterError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, ToolkitError>;
