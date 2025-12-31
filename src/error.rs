//! Error types for solana-pipkit.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ToolkitError>;

#[derive(Error, Debug)]
pub enum ToolkitError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Signature error: {0}")]
    SignatureError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Safety check failed: {0}")]
    SafetyCheckFailed(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Custom error: {0}")]
    Custom(String),

    #[error("Jupiter error: {0}")]
    JupiterError(String),

    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("Amount validation failed: {reason}")]
    AmountValidation { reason: String },

    #[error("Invalid address '{address}': {reason}")]
    InvalidAddress { address: String, reason: String },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
}

impl ToolkitError {
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        ToolkitError::Custom(msg.into())
    }
}

impl From<std::io::Error> for ToolkitError {
    fn from(err: std::io::Error) -> Self {
        ToolkitError::Unknown(err.to_string())
    }
}

impl From<solana_sdk::pubkey::ParsePubkeyError> for ToolkitError {
    fn from(err: solana_sdk::pubkey::ParsePubkeyError) -> Self {
        ToolkitError::ParseError(err.to_string())
    }
}

impl From<solana_client::client_error::ClientError> for ToolkitError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        ToolkitError::RpcError(err.to_string())
    }
}

impl From<bs58::decode::Error> for ToolkitError {
    fn from(err: bs58::decode::Error) -> Self {
        ToolkitError::ParseError(err.to_string())
    }
}

impl From<solana_sdk::program_error::ProgramError> for ToolkitError {
    fn from(err: solana_sdk::program_error::ProgramError) -> Self {
        ToolkitError::TokenError(err.to_string())
    }
}

impl From<bincode::Error> for ToolkitError {
    fn from(err: bincode::Error) -> Self {
        ToolkitError::SerializationError(err.to_string())
    }
}
