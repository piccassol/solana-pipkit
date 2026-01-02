//! Type definitions for agent registry.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

/// Maximum number of capabilities an agent can register.
pub const MAX_CAPABILITIES: usize = 32;

/// Maximum size of agent metadata in bytes.
pub const MAX_METADATA_SIZE: usize = 1024;

/// Maximum version string length.
pub const MAX_VERSION_LENGTH: usize = 64;

/// Maximum number of agents to return in a single query.
pub const MAX_QUERY_LIMIT: usize = 100;

/// On-chain agent registry account data.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AgentAccount {
    /// Agent's pubkey.
    pub pubkey: Pubkey,
    /// Owner/signer of this agent registration.
    pub owner: Pubkey,
    /// List of capabilities offered by this agent.
    pub capabilities: Vec<String>,
    /// Agent software version (e.g., "1.0.0").
    pub version: String,
    /// Optional metadata (JSON or custom binary).
    pub metadata: Vec<u8>,
    /// Registration timestamp (Unix timestamp).
    pub registration_time: i64,
    /// Last update timestamp.
    pub last_update_time: i64,
    /// Whether the agent is currently active.
    pub active: bool,
    /// Agent reputation score (0-10000, representing 0.00-100.00).
    pub reputation: u64,
    /// Bump seed for PDA derivation.
    pub bump: u8,
}

impl AgentAccount {
    /// Validate agent account data.
    pub fn validate(&self) -> Result<(), crate::agent_registry::errors::AgentRegistryError> {
        if self.capabilities.len() > MAX_CAPABILITIES {
            return Err(
                crate::agent_registry::errors::AgentRegistryError::TooManyCapabilities {
                    max: MAX_CAPABILITIES,
                    provided: self.capabilities.len(),
                },
            );
        }

        if self.metadata.len() > MAX_METADATA_SIZE {
            return Err(
                crate::agent_registry::errors::AgentRegistryError::MetadataTooLarge {
                    max: MAX_METADATA_SIZE,
                    provided: self.metadata.len(),
                },
            );
        }

        if self.version.len() > MAX_VERSION_LENGTH {
            return Err(
                crate::agent_registry::errors::AgentRegistryError::InvalidVersion(
                    "Version string too long".to_string(),
                ),
            );
        }

        for cap in &self.capabilities {
            if cap.is_empty() || cap.len() > 128 {
                return Err(
                    crate::agent_registry::errors::AgentRegistryError::InvalidCapability(
                        cap.clone(),
                    ),
                );
            }
        }

        Ok(())
    }
}

/// Lightweight agent information for query results.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent's pubkey.
    pub pubkey: Pubkey,
    /// Owner/signer of this agent.
    pub owner: Pubkey,
    /// Set of capabilities.
    pub capabilities: HashSet<String>,
    /// Agent version.
    pub version: String,
    /// Metadata (if any).
    pub metadata: Option<Vec<u8>>,
    /// Registration timestamp.
    pub registration_time: i64,
    /// Last update timestamp.
    pub last_update_time: i64,
    /// Whether the agent is active.
    pub active: bool,
    /// Reputation score (0-10000).
    pub reputation: u64,
}

impl From<AgentAccount> for AgentInfo {
    fn from(account: AgentAccount) -> Self {
        Self {
            pubkey: account.pubkey,
            owner: account.owner,
            capabilities: account.capabilities.into_iter().collect(),
            version: account.version,
            metadata: if account.metadata.is_empty() {
                None
            } else {
                Some(account.metadata)
            },
            registration_time: account.registration_time,
            last_update_time: account.last_update_time,
            active: account.active,
            reputation: account.reputation,
        }
    }
}

/// Agent capability descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Capability {
    /// Capability name (e.g., "inference", "training", "storage").
    pub name: String,
    /// Optional capability version.
    pub version: Option<String>,
    /// Optional parameters for this capability.
    pub params: Option<serde_json::Value>,
}

impl Capability {
    /// Create a new capability.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: None,
            params: None,
        }
    }

    /// Set the capability version.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set capability parameters.
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// Agent registration parameters.
#[derive(Debug, Clone)]
pub struct RegistrationParams {
    /// Agent owner/signer.
    pub owner: Pubkey,
    /// List of capabilities.
    pub capabilities: Vec<String>,
    /// Agent version.
    pub version: String,
    /// Optional metadata.
    pub metadata: Option<Vec<u8>>,
}

impl RegistrationParams {
    /// Create new registration parameters.
    pub fn new(owner: Pubkey, capabilities: Vec<String>, version: &str) -> Self {
        Self {
            owner,
            capabilities,
            version: version.to_string(),
            metadata: None,
        }
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: Vec<u8>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Agent update parameters.
#[derive(Debug, Clone)]
pub struct UpdateParams {
    /// New capabilities (optional).
    pub capabilities: Option<Vec<String>>,
    /// New version (optional).
    pub version: Option<String>,
    /// New metadata (optional).
    pub metadata: Option<Vec<u8>>,
    /// Set active status (optional).
    pub active: Option<bool>,
}

impl UpdateParams {
    /// Create empty update parameters.
    pub fn new() -> Self {
        Self {
            capabilities: None,
            version: None,
            metadata: None,
            active: None,
        }
    }

    /// Set new capabilities.
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Set new version.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set new metadata.
    pub fn with_metadata(mut self, metadata: Vec<u8>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set active status.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }
}

/// Query filter for agent searches.
#[derive(Debug, Clone, Default)]
pub struct AgentFilter {
    /// Filter by capability (exact match).
    pub capability: Option<String>,
    /// Filter by minimum reputation.
    pub min_reputation: Option<u64>,
    /// Filter by active status.
    pub active_only: bool,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: usize,
}

impl AgentFilter {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by capability.
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capability = Some(capability.to_string());
        self
    }

    /// Filter by minimum reputation.
    pub fn with_min_reputation(mut self, reputation: u64) -> Self {
        self.min_reputation = Some(reputation);
        self
    }

    /// Filter only active agents.
    pub fn active_only(mut self) -> Self {
        self.active_only = true;
        self
    }

    /// Set result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit.min(MAX_QUERY_LIMIT));
        self
    }

    /// Set pagination offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

/// Registry event types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryEvent {
    /// Agent registered.
    AgentRegistered { agent: Pubkey, owner: Pubkey },
    /// Agent updated.
    AgentUpdated { agent: Pubkey },
    /// Agent deregistered/deactivated.
    AgentDeregistered { agent: Pubkey },
    /// Agent capability changed.
    CapabilityChanged { agent: Pubkey, capability: String },
}

/// Pagination parameters.
#[derive(Debug, Clone, Default)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Pagination result.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Total number of items.
    pub total: usize,
    /// Whether there are more items.
    pub has_more: bool,
    /// Current offset.
    pub offset: usize,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result.
    pub fn new(items: Vec<T>, total: usize, offset: usize) -> Self {
        let has_more = offset + items.len() < total;
        Self {
            items,
            total,
            has_more,
            offset,
        }
    }

    /// Get the next page offset.
    pub fn next_offset(&self) -> Option<usize> {
        if self.has_more {
            Some(self.offset + self.items.len())
        } else {
            None
        }
    }
}
