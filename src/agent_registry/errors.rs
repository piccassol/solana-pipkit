//! Error types for agent registry operations.

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgentRegistryError>;

/// Errors that can occur during agent registry operations.
#[derive(Error, Debug)]
pub enum AgentRegistryError {
    #[error("Agent not found: {0}")]
    AgentNotFound(Pubkey),

    #[error("Agent already registered: {0}")]
    AgentAlreadyRegistered(Pubkey),

    #[error("Invalid agent owner: {0}")]
    InvalidAgentOwner(Pubkey),

    #[error("Invalid capability: {0}")]
    InvalidCapability(String),

    #[error("Too many capabilities: maximum {max}, provided {provided}")]
    TooManyCapabilities { max: usize, provided: usize },

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Metadata too large: maximum {max} bytes, provided {provided}")]
    MetadataTooLarge { max: usize, provided: usize },

    #[error("Insufficient rent exemption: required {required} lamports, provided {provided}")]
    InsufficientRentExemption { required: u64, provided: u64 },

    #[error("Account not rent exempt: {0}")]
    AccountNotRentExempt(Pubkey),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("PDA derivation failed")]
    PdaDerivationFailed,

    #[error("Invalid program ID: {0}")]
    InvalidProgramId(Pubkey),

    #[error("Feature not supported: {0}")]
    FeatureNotSupported(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),
}

impl From<solana_client::client_error::ClientError> for AgentRegistryError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        AgentRegistryError::RpcError(err.to_string())
    }
}

impl From<bincode::Error> for AgentRegistryError {
    fn from(err: bincode::Error) -> Self {
        AgentRegistryError::SerializationError(err.to_string())
    }
}

impl From<borsh::io::Error> for AgentRegistryError {
    fn from(err: borsh::io::Error) -> Self {
        AgentRegistryError::SerializationError(err.to_string())
    }
}
