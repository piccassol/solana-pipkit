//! Client implementation for agent registry interactions.
//!
//! This module provides instruction builders, RPC query helpers, and
//! utility functions for interacting with the on-chain agent registry.

use super::errors::{AgentRegistryError, Result};
use super::types::{
    AgentAccount, AgentFilter, AgentInfo, PaginatedResult, RegistryEvent, RegistrationParams,
    UpdateParams,
};
use crate::agent_registry::types::{MAX_METADATA_SIZE, MAX_VERSION_LENGTH};
use borsh::BorshDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;

/// Agent registry program ID.
/// Replace with your deployed program ID.
pub const AGENT_REGISTRY_PROGRAM_ID: Pubkey = solana_sdk::system_program::id();

/// PDA seed prefix for agent accounts.
pub const AGENT_SEED: &[u8] = b"agent";

/// Instruction discriminator for register_agent.
pub const INSTRUCTION_REGISTER_AGENT: u8 = 0;

/// Instruction discriminator for update_agent.
pub const INSTRUCTION_UPDATE_AGENT: u8 = 1;

/// Instruction discriminator for deregister_agent.
pub const INSTRUCTION_DEREGISTER_AGENT: u8 = 2;

/// Client for interacting with the agent registry.
pub struct AgentRegistryClient {
    /// RPC client.
    pub client: Arc<RpcClient>,
    /// Program ID (can be customized for testing).
    pub program_id: Pubkey,
}

impl Clone for AgentRegistryClient {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            program_id: self.program_id,
        }
    }
}

impl AgentRegistryClient {
    /// Create a new agent registry client.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The RPC endpoint URL
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::AgentRegistryClient;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
    /// }
    /// ```
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Arc::new(RpcClient::new(rpc_url)),
            program_id: AGENT_REGISTRY_PROGRAM_ID,
        }
    }

    /// Create a client with a custom program ID.
    pub fn with_program_id(rpc_url: String, program_id: Pubkey) -> Self {
        Self {
            client: Arc::new(RpcClient::new(rpc_url)),
            program_id,
        }
    }

    /// Derive the agent PDA for a given agent pubkey.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    ///
    /// # Returns
    ///
    /// The PDA address and bump seed.
    pub fn derive_agent_pda(&self, agent_pubkey: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[AGENT_SEED, agent_pubkey.as_ref()], &self.program_id)
    }

    /// Create a register_agent instruction.
    ///
    /// # Arguments
    ///
    /// * `params` - Registration parameters
    ///
    /// # Returns
    ///
    /// A Solana instruction to register an agent.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::{AgentRegistryClient, RegistrationParams};
    /// use solana_sdk::{signature::Keypair, pubkey::Pubkey};
    ///
    /// let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
    /// let agent = Keypair::new();
    /// let params = RegistrationParams::new(
    ///     agent.pubkey(),
    ///     vec!["inference".to_string(), "training".to_string()],
    ///     "1.0.0"
    /// );
    /// let instruction = client.register_agent_instruction(&params).unwrap();
    /// ```
    pub fn register_agent_instruction(&self, params: &RegistrationParams) -> Result<Instruction> {
        let agent_pubkey = params.owner;
        let (agent_pda, bump) = self.derive_agent_pda(&agent_pubkey);

        if params.capabilities.len() > 32 {
            return Err(AgentRegistryError::TooManyCapabilities {
                max: 32,
                provided: params.capabilities.len(),
            });
        }

        if params.version.len() > MAX_VERSION_LENGTH {
            return Err(AgentRegistryError::InvalidVersion(
                "Version string too long".to_string(),
            ));
        }

        let metadata = params.metadata.clone().unwrap_or_default();

        if metadata.len() > MAX_METADATA_SIZE {
            return Err(AgentRegistryError::MetadataTooLarge {
                max: MAX_METADATA_SIZE,
                provided: metadata.len(),
            });
        }

        let mut data = vec![INSTRUCTION_REGISTER_AGENT, bump];
        data.extend_from_slice(&(params.capabilities.len() as u8).to_le_bytes());

        for cap in &params.capabilities {
            data.extend_from_slice(&(cap.len() as u8).to_le_bytes());
            data.extend_from_slice(cap.as_bytes());
        }

        data.extend_from_slice(&(params.version.len() as u8).to_le_bytes());
        data.extend_from_slice(params.version.as_bytes());

        data.extend_from_slice(&(metadata.len() as u16).to_le_bytes());
        data.extend_from_slice(&metadata);

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(agent_pda, false),
                AccountMeta::new_readonly(agent_pubkey, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        })
    }

    /// Create an update_agent instruction.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    /// * `params` - Update parameters
    ///
    /// # Returns
    ///
    /// A Solana instruction to update an agent.
    pub fn update_agent_instruction(
        &self,
        agent_pubkey: &Pubkey,
        params: &UpdateParams,
    ) -> Result<Instruction> {
        let (agent_pda, bump) = self.derive_agent_pda(agent_pubkey);

        let mut data = vec![INSTRUCTION_UPDATE_AGENT, bump];

        let has_capabilities = params.capabilities.is_some();
        let has_version = params.version.is_some();
        let has_metadata = params.metadata.is_some();
        let has_active = params.active.is_some();

        let mut flags: u8 = 0;
        if has_capabilities {
            flags |= 0b0001;
        }
        if has_version {
            flags |= 0b0010;
        }
        if has_metadata {
            flags |= 0b0100;
        }
        if has_active {
            flags |= 0b1000;
        }

        data.push(flags);

        if let Some(caps) = &params.capabilities {
            data.extend_from_slice(&(caps.len() as u8).to_le_bytes());
            for cap in caps {
                data.extend_from_slice(&(cap.len() as u8).to_le_bytes());
                data.extend_from_slice(cap.as_bytes());
            }
        }

        if let Some(version) = &params.version {
            data.extend_from_slice(&(version.len() as u8).to_le_bytes());
            data.extend_from_slice(version.as_bytes());
        }

        if let Some(metadata) = &params.metadata {
            data.extend_from_slice(&(metadata.len() as u16).to_le_bytes());
            data.extend_from_slice(metadata);
        }

        if let Some(active) = params.active {
            data.push(if active { 1 } else { 0 });
        }

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(agent_pda, false),
                AccountMeta::new_readonly(*agent_pubkey, true),
            ],
            data,
        })
    }

    /// Create a deregister_agent instruction.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    ///
    /// # Returns
    ///
    /// A Solana instruction to deregister an agent.
    pub fn deregister_agent_instruction(&self, agent_pubkey: &Pubkey) -> Result<Instruction> {
        let (agent_pda, bump) = self.derive_agent_pda(agent_pubkey);

        let data = vec![INSTRUCTION_DEREGISTER_AGENT, bump];

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(agent_pda, false),
                AccountMeta::new_readonly(*agent_pubkey, true),
            ],
            data,
        })
    }

    /// Register an agent on-chain.
    ///
    /// # Arguments
    ///
    /// * `params` - Registration parameters
    /// * `payer` - The payer keypair (must match agent owner)
    ///
    /// # Returns
    ///
    /// The transaction signature.
    pub async fn register_agent(
        &self,
        params: &RegistrationParams,
        payer: &Keypair,
    ) -> Result<solana_sdk::signature::Signature> {
        let instruction = self.register_agent_instruction(params)?;
        let recent_blockhash = self.client.get_latest_blockhash().await?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| AgentRegistryError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Update an agent's information on-chain.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    /// * `params` - Update parameters
    /// * `payer` - The agent owner keypair
    ///
    /// # Returns
    ///
    /// The transaction signature.
    pub async fn update_agent(
        &self,
        agent_pubkey: &Pubkey,
        params: &UpdateParams,
        payer: &Keypair,
    ) -> Result<solana_sdk::signature::Signature> {
        let instruction = self.update_agent_instruction(agent_pubkey, params)?;
        let recent_blockhash = self.client.get_latest_blockhash().await?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| AgentRegistryError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Deregister an agent from the registry.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    /// * `payer` - The agent owner keypair
    ///
    /// # Returns
    ///
    /// The transaction signature.
    pub async fn deregister_agent(
        &self,
        agent_pubkey: &Pubkey,
        payer: &Keypair,
    ) -> Result<solana_sdk::signature::Signature> {
        let instruction = self.deregister_agent_instruction(agent_pubkey)?;
        let recent_blockhash = self.client.get_latest_blockhash().await?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| AgentRegistryError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Fetch agent information by pubkey.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    ///
    /// # Returns
    ///
    /// Agent information if found.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::AgentRegistryClient;
    /// use solana_sdk::pubkey::Pubkey;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
    ///     let agent_pubkey = Pubkey::from_str("...").unwrap();
    ///     let agent = client.fetch_agent(&agent_pubkey).await.unwrap();
    ///     println!("Agent: {:?}", agent);
    /// }
    /// ```
    pub async fn fetch_agent(&self, agent_pubkey: &Pubkey) -> Result<AgentInfo> {
        let (agent_pda, _) = self.derive_agent_pda(agent_pubkey);

        let account = self
            .client
            .get_account(&agent_pda)
            .await
            .map_err(|e| {
                if e.to_string().contains("not found") {
                    AgentRegistryError::AgentNotFound(*agent_pubkey)
                } else {
                    AgentRegistryError::RpcError(e.to_string())
                }
            })?;

        let agent_account: AgentAccount =
            AgentAccount::try_from_slice(&account.data).map_err(|e| {
                AgentRegistryError::InvalidAccountData(format!("Failed to deserialize: {}", e))
            })?;

        Ok(agent_account.into())
    }

    /// Check if an agent account is rent-exempt.
    ///
    /// # Arguments
    ///
    /// * `agent_pubkey` - The agent's pubkey
    ///
    /// # Returns
    ///
    /// True if the account is rent-exempt.
    pub async fn is_rent_exempt(&self, agent_pubkey: &Pubkey) -> Result<bool> {
        let (agent_pda, _) = self.derive_agent_pda(agent_pubkey);

        match self.client.get_account(&agent_pda).await {
            Ok(account) => {
                Ok(account.lamports >= self.get_minimum_balance_for_rent_exemption().await?)
            }
            Err(_) => Ok(false),
        }
    }

    /// Get minimum balance for rent exemption.
    pub async fn get_minimum_balance_for_rent_exemption(&self) -> Result<u64> {
        let account_data = AgentAccount {
            pubkey: Pubkey::default(),
            owner: Pubkey::default(),
            capabilities: vec![],
            version: String::new(),
            metadata: vec![],
            registration_time: 0,
            last_update_time: 0,
            active: false,
            reputation: 0,
            bump: 0,
        };

        let data = borsh::to_vec(&account_data).map_err(|e| {
            AgentRegistryError::SerializationError(format!("Failed to serialize: {}", e))
        })?;

        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(data.len())
            .await
            .map_err(|e| AgentRegistryError::RpcError(e.to_string()))?;

        Ok(lamports)
    }

    /// Search for agents by capability.
    ///
    /// # Arguments
    ///
    /// * `capability` - The capability to search for
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    ///
    /// A vector of agents with the specified capability.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::AgentRegistryClient;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
    ///     let agents = client.search_agents_by_capability("inference", 10).await.unwrap();
    ///     println!("Found {} inference agents", agents.len());
    /// }
    /// ```
    pub async fn search_agents_by_capability(
        &self,
        capability: &str,
        limit: usize,
    ) -> Result<Vec<AgentInfo>> {
        let accounts = self
            .client
            .get_program_accounts_with_config(
                &self.program_id,
                solana_client::rpc_config::RpcProgramAccountsConfig {
                    account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                        encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    filters: None,
                    with_context: None,
                },
            )
            .await
            .map_err(|e| AgentRegistryError::RpcError(e.to_string()))?;

        let mut results = Vec::new();

        for (_pubkey, account) in accounts {
            if results.len() >= limit {
                break;
            }

            let agent_account: AgentAccount =
                AgentAccount::try_from_slice(&account.data).map_err(|e| {
                    AgentRegistryError::InvalidAccountData(format!("Failed to deserialize: {}", e))
                })?;

            if agent_account.active && agent_account.capabilities.contains(&capability.to_string())
            {
                results.push(agent_account.into());
            }
        }

        Ok(results)
    }

    /// List all registered agents with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `filter` - Query filters
    ///
    /// # Returns
    ///
    /// A paginated result of agents.
    pub async fn list_agents(&self, filter: &AgentFilter) -> Result<PaginatedResult<AgentInfo>> {
        let limit = filter.limit.unwrap_or(50).min(100);

        let accounts = self
            .client
            .get_program_accounts_with_config(
                &self.program_id,
                solana_client::rpc_config::RpcProgramAccountsConfig {
                    account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                        encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    filters: None,
                    with_context: None,
                },
            )
            .await
            .map_err(|e| AgentRegistryError::RpcError(e.to_string()))?;

        let mut all_agents: Vec<AgentInfo> = Vec::new();

        for (_pubkey, account) in accounts {
            let agent_account: AgentAccount =
                AgentAccount::try_from_slice(&account.data).map_err(|e| {
                    AgentRegistryError::InvalidAccountData(format!("Failed to deserialize: {}", e))
                })?;

            if filter.active_only && !agent_account.active {
                continue;
            }

            if let Some(min_rep) = filter.min_reputation {
                if agent_account.reputation < min_rep {
                    continue;
                }
            }

            if let Some(ref cap) = filter.capability {
                if !agent_account.capabilities.contains(cap) {
                    continue;
                }
            }

            all_agents.push(agent_account.into());
        }

        let total = all_agents.len();
        let _end_offset = (filter.offset + limit).min(total);
        let items: Vec<AgentInfo> = all_agents
            .into_iter()
            .skip(filter.offset)
            .take(limit)
            .collect();

        Ok(PaginatedResult::new(items, total, filter.offset))
    }

    /// Get the total count of registered agents.
    pub async fn get_agent_count(&self) -> Result<usize> {
        let accounts = self
            .client
            .get_program_accounts(&self.program_id)
            .await
            .map_err(|e| AgentRegistryError::RpcError(e.to_string()))?;

        Ok(accounts.len())
    }

    /// Discover peers matching specific capabilities for task routing.
    ///
    /// This integrates with the multi_agent module to find suitable agents
    /// for collaborative tasks.
    ///
    /// # Arguments
    ///
    /// * `required_capabilities` - List of required capabilities
    /// * `min_reputation` - Minimum reputation threshold
    /// * `limit` - Maximum number of peers to return
    ///
    /// # Returns
    ///
    /// A map of capability to list of agents with that capability.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::AgentRegistryClient;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
    ///     let peers = client.discover_peers(
    ///         vec!["inference".to_string(), "training".to_string()],
    ///         5000,
    ///         10
    ///     ).await.unwrap();
    ///     
    ///     for (capability, agents) in peers {
    ///         println!("{}: {} agents available", capability, agents.len());
    ///     }
    /// }
    /// ```
    pub async fn discover_peers(
        &self,
        required_capabilities: Vec<String>,
        min_reputation: u64,
        limit: usize,
    ) -> Result<HashMap<String, Vec<AgentInfo>>> {
        let mut result: HashMap<String, Vec<AgentInfo>> = HashMap::new();

        for capability in &required_capabilities {
            let agents = self
                .search_agents_by_capability(capability, limit)
                .await?
                .into_iter()
                .filter(|agent| agent.active && agent.reputation >= min_reputation)
                .collect();

            result.insert(capability.clone(), agents);
        }

        Ok(result)
    }

    /// Subscribe to registry events via WebSocket.
    ///
    /// This provides real-time updates when agents register, update, or deregister.
    ///
    /// # Arguments
    ///
    /// * `callback` - Async callback function to handle events
    ///
    /// # Returns
    ///
    /// A handle that can be used to stop the subscription.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use solana_pipkit::agent_registry::{AgentRegistryClient, RegistryEvent};
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = AgentRegistryClient::new("wss://api.mainnet-beta.solana.com".to_string());
    ///
    ///     let (tx, mut rx) = mpsc::unbounded_channel();
    ///     let handle = client.subscribe_to_registry_events(move |event| {
    ///         let tx = tx.clone();
    ///         async move {
    ///             let _ = tx.send(event);
    ///         }
    ///     }).await.unwrap();
    ///
    ///     while let Some(event) = rx.recv().await {
    ///         println!("Event: {:?}", event);
    ///     }
    /// }
    /// ```
    pub async fn subscribe_to_registry_events<F, Fut>(
        &self,
        mut callback: F,
    ) -> Result<tokio::task::JoinHandle<()>>
    where
        F: FnMut(RegistryEvent) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let client_clone = Arc::clone(&self.client);

        let handle = tokio::spawn(async move {
            use borsh::BorshDeserialize;

            let mut known_agents: HashMap<Pubkey, AgentAccount> = HashMap::new();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                match client_clone.get_program_accounts(&AGENT_REGISTRY_PROGRAM_ID).await {
                Ok(accounts) => {
                    let mut current_agents: HashMap<Pubkey, AgentAccount> = HashMap::new();

                    for (pubkey, account) in accounts {
                        if let Ok(agent_account) = AgentAccount::try_from_slice(&account.data) {
                            current_agents.insert(pubkey, agent_account);
                        }
                    }

                    for (pubkey, agent_account) in &current_agents {
                        if !known_agents.contains_key(pubkey) {
                            let event = RegistryEvent::AgentRegistered {
                                agent: *pubkey,
                                owner: agent_account.owner,
                            };
                            callback(event).await;
                        } else {
                            let prev = known_agents.get(pubkey).unwrap();
                            if prev.capabilities != agent_account.capabilities {
                                for new_cap in &agent_account.capabilities {
                                    if !prev.capabilities.contains(new_cap) {
                                        let event = RegistryEvent::CapabilityChanged {
                                            agent: *pubkey,
                                            capability: new_cap.clone(),
                                        };
                                        callback(event).await;
                                    }
                                }
                            }
                        }
                    }

                    for pubkey in known_agents.keys() {
                        if !current_agents.contains_key(pubkey) {
                            let event = RegistryEvent::AgentDeregistered { agent: *pubkey };
                            callback(event).await;
                        }
                    }

                    known_agents = current_agents;
                }
                Err(e) => {
                        eprintln!("Error polling registry: {}", e);
                    }
                }
            }
        });

        Ok(handle)
    }
}
