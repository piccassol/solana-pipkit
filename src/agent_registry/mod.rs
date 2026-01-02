//! Agent Registry Module for AgenC-style multi-agent coordination.
//!
//! This module provides client-side utilities for interacting with an on-chain
//! agent registry program, enabling decentralized agent discovery and coordination
//! for lightweight AI agents running on edge devices.
//!
//! # Features
//!
//! - **Agent Registration**: Register agents with capabilities, version, and metadata
//! - **Agent Discovery**: Search and filter agents by capability, reputation, and status
//! - **Event Monitoring**: Real-time subscription to registry changes
//! - **Multi-Agent Integration**: Peer discovery for collaborative tasks
//! - **Safety Checks**: Rent exemption validation, metadata size limits, signature verification
//!
//! # Example: AgenC Edge Agent Registration
//!
//! This example shows how an AgenC edge agent would register itself on boot
//! and later discover peers for collaborative tasks.
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::{AgentRegistryClient, RegistrationParams};
//! use solana_sdk::{signature::Keypair, pubkey::Pubkey};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the agent registry client
//!     let client = AgentRegistryClient::new(
//!         "https://api.mainnet-beta.solana.com".to_string()
//!     );
//!
//!     // Generate agent keypair (on edge device, this would be persistent storage)
//!     let agent = Keypair::new();
//!
//!     // Define agent capabilities
//!     let capabilities = vec![
//!         "inference".to_string(),
//!         "model_finetuning".to_string(),
//!         "edge_computing".to_string(),
//!     ];
//!
//!     // Prepare registration parameters
//!     let params = RegistrationParams::new(
//!         agent.pubkey(),
//!         capabilities,
//!         "1.0.0"
//!     ).with_metadata(serde_json::json!({
//!         "device_type": "jetson-nano",
//!         "gpu": "aarch64",
//!         "max_batch_size": 32,
//!         "supported_models": vec!["gpt-2", "bert"]
//!     }).to_vec());
//!
//!     // Register the agent
//!     println!("Registering agent {}...", agent.pubkey());
//!     let signature = client.register_agent(&params, &agent).await?;
//!     println!("Registered! Signature: {}", signature);
//!
//!     // Discover peers for collaborative inference
//!     println!("\nDiscovering peers for collaborative inference...");
//!     let peers = client.discover_peers(
//!         vec!["inference".to_string()],
//!         7000, // minimum reputation (70.00)
//!         10
//!     ).await?;
//!
//!     if let Some(inference_agents) = peers.get("inference") {
//!         println!("Found {} inference agents:", inference_agents.len());
//!         for peer in inference_agents {
//!             println!("  - {} (reputation: {})", peer.pubkey, peer.reputation);
//!         }
//!     }
//!
//!     // Subscribe to registry events
//!     println!("\nSubscribing to registry events...");
//!     let event_handle = client.subscribe_to_registry_events(|event| async move {
//!         match event {
//!             solana_pipkit::agent_registry::RegistryEvent::AgentRegistered { agent, owner } => {
//!                 println!("New agent registered: {} (owner: {})", agent, owner);
//!             }
//!             solana_pipkit::agent_registry::RegistryEvent::AgentUpdated { agent } => {
//!                 println!("Agent updated: {}", agent);
//!             }
//!             _ => {}
//!         }
//!     }).await?;
//!
//!     // In production, run indefinitely
//!     tokio::time::sleep(Duration::from_secs(60)).await;
//!     
//!     event_handle.abort();
//!
//!     Ok(())
//! }
//! ```
//!
//! # Program Assumptions
//!
//! This module assumes an Anchor-based on-chain program with the following structure:
//!
//! ## Account Structure
//!
//! ```ignore
//! pub struct Agent {
//!     pub pubkey: Pubkey,
//!     pub owner: Pubkey,
//!     pub capabilities: Vec<String>,
//!     pub version: String,
//!     pub metadata: Vec<u8>,
//!     pub registration_time: i64,
//!     pub last_update_time: i64,
//!     pub active: bool,
//!     pub reputation: u64,
//!     pub bump: u8,
//! }
//! ```
//!
//! ## Instructions
//!
//! - `register_agent`: Initialize agent account with PDA ["agent", agent_pubkey]
//! - `update_agent`: Update capabilities, version, metadata, or active status
//! - `deregister_agent`: Mark agent as inactive (or close account)
//!
//! # Usage Patterns
//!
//! ## Boot Registration
//!
//! Edge devices should register themselves on startup:
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::{AgentRegistryClient, RegistrationParams};
//! use solana_sdk::signature::Keypair;
//!
//! async fn register_on_boot(agent: &Keypair) -> Result<(), Box<dyn std::error::Error>> {
//!     let client = AgentRegistryClient::new("https://api.mainnet-beta.solana.com".to_string());
//!     
//!     let params = RegistrationParams::new(
//!         agent.pubkey(),
//!         vec!["inference".to_string()],
//!         env!("CARGO_PKG_VERSION")
//!     );
//!     
//!     client.register_agent(&params, agent).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Peer Discovery for Tasks
//!
//! When coordinating collaborative tasks:
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::AgentRegistryClient;
//! use solana_sdk::pubkey::Pubkey;
//!
//! async fn find_training_partners(
//!     client: &AgentRegistryClient,
//!     required_reputation: u64,
//! ) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
//!     let peers = client.discover_peers(
//!         vec!["model_finetuning".to_string()],
//!         required_reputation,
//!         5
//!     ).await?;
//!
//!     if let Some(training_agents) = peers.get("model_finetuning") {
//!         Ok(training_agents.iter().map(|a| a.pubkey).collect())
//!     } else {
//!         Ok(vec![])
//!     }
//! }
//! ```
//!
//! ## Event-Driven Coordination
//!
//! Monitor registry changes for real-time peer discovery:
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::{AgentRegistryClient, RegistryEvent};
//!
//! async fn monitor_for_new_capabilities(client: AgentRegistryClient) {
//!     let _handle = client.subscribe_to_registry_events(|event| async move {
//!         if let RegistryEvent::CapabilityChanged { agent, capability } = event {
//!             if capability == "inference" {
//!                 println!("New inference agent available: {}", agent);
//!             }
//!         }
//!     }).await;
//! }
//! ```
//!
//! # Error Handling
//!
//! The module uses a custom `AgentRegistryError` type with comprehensive error cases:
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::{AgentRegistryClient, AgentRegistryError};
//!
//! async fn safe_registration(client: &AgentRegistryClient) -> Result<(), AgentRegistryError> {
//!     // Handle common error cases
//!     match client.fetch_agent(&pubkey).await {
//!         Ok(_) => Err(AgentRegistryError::AgentAlreadyRegistered(pubkey)),
//!         Err(AgentRegistryError::AgentNotFound(_)) => {
//!             // Safe to proceed with registration
//!             Ok(())
//!         }
//!         Err(e) => Err(e),
//!     }
//! }
//! ```
//!
//! # Integration with Multi-Agent Module
//!
//! This module integrates with the `multi_agent` module for end-to-end agent coordination:
//!
//! ```rust,no_run
//! use solana_pipkit::agent_registry::AgentRegistryClient;
//! use solana_pipkit::multi_agent::TaskMarketplace;
//!
//! async fn coordinate_task() {
//!     let registry = AgentRegistryClient::new("https://api.devnet.solana.com".to_string());
//!     let marketplace = TaskMarketplace::new(registry.client.clone());
//!     
//!     // Discover capable agents
//!     let peers = registry.discover_peers(
//!         vec!["inference".to_string()],
//!         8000,
//!         5
//!     ).await.unwrap();
//!     
//!     // Create task for marketplace
//!     for (_, agents) in peers {
//!         for agent in agents {
//!             // Post task to agent's inbox via marketplace
//!         }
//!     }
//! }
//! ```

pub mod client;
pub mod errors;
pub mod types;

pub use client::{AgentRegistryClient, AGENT_REGISTRY_PROGRAM_ID, AGENT_SEED};
pub use errors::{AgentRegistryError, Result};
pub use types::{
    AgentAccount, AgentFilter, AgentInfo, Capability, PaginatedResult, PaginationParams,
    RegistryEvent, RegistrationParams, UpdateParams, MAX_CAPABILITIES, MAX_METADATA_SIZE,
    MAX_QUERY_LIMIT, MAX_VERSION_LENGTH,
};
