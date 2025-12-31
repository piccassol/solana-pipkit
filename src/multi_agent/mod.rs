//! Multi-Agent Coordination primitives for Solana.
//!
//! This module provides secure, low-latency, decentralized coordination primitives
//! for multiple Solana agents, designed for AgenC-style multi-agent systems.
//!
//! # Features
//!
//! - PDA-derived agent inboxes for secure message routing
//! - On-chain message posting with optional encryption
//! - Task marketplace bidding instructions
//! - Shared event subscriptions via WebSocket/Geyser
//! - Encryption helpers for private agent-to-agent communication
//!
//! # Example
//!
//! ```rust,no_run
//! use solana_pipkit::multi_agent::*;
//! use solana_sdk::{signature::Keypair, pubkey::Pubkey};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let agent = Keypair::new();
//!     let target = Pubkey::from_str("TargetAddress...")?;
//!
//!     // Derive agent inbox
//!     let inbox = AgentInbox::derive(&agent.pubkey());
//!
//!     // Post a message
//!     let instruction = post_message(&inbox, &agent.pubkey(), b"Hello!".to_vec());
//!
//!     // Fetch messages
//!     let messages = fetch_inbox_messages(&inbox, None).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::{Result, ToolkitError};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub mod encryption;
pub mod event_stream;
pub mod marketplace;

pub use encryption::{EncryptionKey, MessageEncryptor};
pub use event_stream::{EventFilter, EventStream, EventType};
pub use marketplace::{TaskBid, TaskMarketplace};

/// Multi-agent coordination program ID.
/// In production, this would be your deployed multi-agent program.
pub const MULTI_AGENT_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("MultiAgent111111111111111111111111111111111");

/// Agent inbox PDA seed prefix.
pub const INBOX_SEED: &[u8] = b"inbox";

/// Message account PDA seed prefix.
pub const MESSAGE_SEED: &[u8] = b"message";

/// Maximum message size in bytes.
pub const MAX_MESSAGE_SIZE: usize = 1024;

/// Default message TTL in seconds (24 hours).
pub const DEFAULT_MESSAGE_TTL: u64 = 86400;

/// On-chain message account data.
#[derive(Debug, Clone)]
pub struct MessageAccount {
    /// Message sender pubkey.
    pub sender: Pubkey,
    /// Message timestamp (Unix timestamp).
    pub timestamp: i64,
    /// Message payload (can be encrypted).
    pub payload: Vec<u8>,
    /// Whether message has been read.
    pub read: bool,
    /// Message nonce for encryption.
    pub nonce: [u8; 24],
    /// Bump seed for PDA.
    pub bump: u8,
}

/// Deserialized message from inbox.
#[derive(Debug, Clone)]
pub struct Message {
    /// Message signature (unique identifier).
    pub signature: Hash,
    /// Sender pubkey.
    pub sender: Pubkey,
    /// Timestamp.
    pub timestamp: i64,
    /// Decrypted payload (if encryption key provided).
    pub payload: Vec<u8>,
    /// Read status.
    pub read: bool,
}

/// Agent inbox for receiving messages.
#[derive(Debug, Clone)]
pub struct AgentInbox {
    /// Inbox pubkey (PDA).
    pub address: Pubkey,
    /// Owner pubkey.
    pub owner: Pubkey,
}

impl AgentInbox {
    /// Derive the inbox PDA for an agent.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The agent's pubkey
    ///
    /// # Returns
    ///
    /// The inbox pubkey and bump seed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use solana_pipkit::multi_agent::AgentInbox;
    /// use solana_sdk::pubkey::Pubkey;
    ///
    /// let agent = Pubkey::new_unique();
    /// let (inbox, bump) = AgentInbox::derive(&agent);
    /// ```
    pub fn derive(agent_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[INBOX_SEED, agent_id.as_ref()], &MULTI_AGENT_PROGRAM_ID)
    }

    /// Derive with a specific bump seed.
    pub fn derive_with_bump(agent_id: &Pubkey, bump: u8) -> Option<Pubkey> {
        let mut seeds = vec![INBOX_SEED, agent_id.as_ref()];
        let bump_arr = [bump];
        seeds.push(&bump_arr);

        Pubkey::create_program_address(&seeds, &MULTI_AGENT_PROGRAM_ID).ok()
    }

    /// Create a new AgentInbox instance.
    pub fn new(agent_id: &Pubkey) -> Self {
        let (address, _bump) = Self::derive(agent_id);
        Self {
            address,
            owner: *agent_id,
        }
    }

    /// Create from existing inbox address.
    pub fn from_address(inbox_address: &Pubkey, owner: &Pubkey) -> Self {
        Self {
            address: *inbox_address,
            owner: *owner,
        }
    }
}

/// Create an instruction to post a message to an agent's inbox.
///
/// # Arguments
///
/// * `to` - The recipient's inbox address
/// * `from` - The sender's pubkey
/// * `payload` - The message payload (will be encrypted if encryptor provided)
///
/// # Returns
///
/// A Solana instruction that can be included in a transaction.
///
/// # Example
///
/// ```rust
/// use solana_pipkit::multi_agent::post_message;
/// use solana_sdk::pubkey::Pubkey;
///
/// let inbox = Pubkey::new_unique();
/// let sender = Pubkey::new_unique();
/// let instruction = post_message(&inbox, &sender, b"Hello!".to_vec());
/// ```
pub fn post_message(to: &Pubkey, from: &Pubkey, payload: Vec<u8>) -> Instruction {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut data = vec![];
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&payload);

    Instruction {
        program_id: MULTI_AGENT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*to, false),
            AccountMeta::new_readonly(*from, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Create an instruction to post an encrypted message.
pub fn post_encrypted_message(
    to: &Pubkey,
    from: &Pubkey,
    payload: Vec<u8>,
    nonce: [u8; 24],
    encryptor: &MessageEncryptor,
) -> Result<Instruction> {
    let encrypted = encryptor.encrypt(&payload, &nonce)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut data = vec![];
    data.push(1); // Encrypted flag
    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&nonce);
    data.extend_from_slice(&(encrypted.len() as u32).to_le_bytes());
    data.extend_from_slice(&encrypted);

    Ok(Instruction {
        program_id: MULTI_AGENT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*to, false),
            AccountMeta::new_readonly(*from, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    })
}

/// Fetch messages from an agent's inbox.
///
/// # Arguments
///
/// * `inbox` - The inbox address
/// * `since` - Optional Unix timestamp to filter messages after
///
/// # Returns
///
/// A vector of messages from the inbox.
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::multi_agent::fetch_inbox_messages;
/// use solana_sdk::pubkey::Pubkey;
///
/// #[tokio::main]
/// async fn main() {
///     let inbox = Pubkey::new_unique();
///     let messages = fetch_inbox_messages(&inbox, None).await.unwrap();
///     println!("Found {} messages", messages.len());
/// }
/// ```
pub async fn fetch_inbox_messages(
    client: &RpcClient,
    inbox: &Pubkey,
    since: Option<i64>,
) -> Result<Vec<Message>> {
    let signatures = client
        .get_signatures_for_address(inbox)
        .await
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    let mut messages = Vec::new();

    for sig_info in signatures {
        let timestamp = sig_info.block_time.unwrap_or(0);

        if let Some(since_ts) = since {
            if timestamp <= since_ts {
                continue;
            }
        }

        let tx = client
            .get_transaction(&sig_info.signature, solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
                commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            })
            .await
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        if let Some(meta) = tx.transaction.meta() {
            if let Some(err) = &meta.err {
                continue;
            }
        }

        let message = parse_message_from_transaction(&tx)?;
        messages.push(message);

        if messages.len() >= 100 {
            break;
        }
    }

    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(messages)
}

/// Parse a message from a transaction.
fn parse_message_from_transaction(
    _tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Message> {
    Err(ToolkitError::ParseError("Transaction parsing not implemented".to_string()))
}

/// Send a message to an agent's inbox.
pub async fn send_message(
    client: &RpcClient,
    to: &Pubkey,
    payer: &Keypair,
    payload: Vec<u8>,
) -> Result<Hash> {
    let instruction = post_message(to, &payer.pubkey(), payload);
    let recent_blockhash = client.get_latest_blockhash().await?;

    let transaction = Transaction::new(
        &[payer],
        solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
        recent_blockhash,
    );

    let signature = client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

    Ok(signature)
}

/// Send an encrypted message to an agent's inbox.
pub async fn send_encrypted_message(
    client: &RpcClient,
    to: &Pubkey,
    payer: &Keypair,
    payload: Vec<u8>,
    encryptor: &MessageEncryptor,
) -> Result<Hash> {
    let nonce = encryptor.generate_nonce();
    let instruction = post_encrypted_message(to, &payer.pubkey(), payload, nonce, encryptor)?;
    let recent_blockhash = client.get_latest_blockhash().await?;

    let transaction = Transaction::new(
        &[payer],
        solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
        recent_blockhash,
    );

    let signature = client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

    Ok(signature)
}

/// Message stream for real-time inbox updates.
pub struct MessageStream {
    client: RpcClient,
    inbox: Pubkey,
    sender: mpsc::UnboundedSender<Message>,
}

impl MessageStream {
    /// Create a new message stream.
    pub fn new(client: RpcClient, inbox: Pubkey) -> (Self, mpsc::UnboundedReceiver<Message>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let stream = Self {
            client,
            inbox,
            sender,
        };
        (stream, receiver)
    }

    /// Start streaming messages.
    pub async fn start(&self) -> Result<()> {
        let mut last_timestamp = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0) - 60);

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let messages = fetch_inbox_messages(&self.client, &self.inbox, last_timestamp).await?;
            last_timestamp = messages.first().map(|m| m.timestamp);

            for msg in messages {
                let _ = self.sender.send(msg);
            }
        }
    }
}

/// Multi-agent coordinator for managing multiple agents.
pub struct MultiAgentCoordinator {
    agents: HashMap<Pubkey, AgentInbox>,
    client: RpcClient,
}

impl MultiAgentCoordinator {
    /// Create a new coordinator.
    pub fn new(client: RpcClient) -> Self {
        Self {
            agents: HashMap::new(),
            client,
        }
    }

    /// Register an agent.
    pub fn register_agent(&mut self, agent_id: &Pubkey) {
        let inbox = AgentInbox::new(agent_id);
        self.agents.insert(*agent_id, inbox);
    }

    /// Get agent inbox.
    pub fn get_inbox(&self, agent_id: &Pubkey) -> Option<&AgentInbox> {
        self.agents.get(agent_id)
    }

    /// Broadcast message to all agents.
    pub async fn broadcast(
        &self,
        sender: &Keypair,
        payload: Vec<u8>,
    ) -> Result<Vec<Hash>> {
        let mut signatures = Vec::new();

        for (_, inbox) in &self.agents {
            if inbox.owner != sender.pubkey() {
                let sig = send_message(&self.client, &inbox.address, sender, payload.clone()).await?;
                signatures.push(sig);
            }
        }

        Ok(signatures)
    }

    /// Get all registered agents.
    pub fn agents(&self) -> Vec<Pubkey> {
        self.agents.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_inbox_derive() {
        let agent = Pubkey::new_unique();
        let (inbox, bump) = AgentInbox::derive(&agent);

        assert_ne!(inbox, Pubkey::default());
        assert!(bump > 0);
    }

    #[test]
    fn test_post_message_instruction() {
        let to = Pubkey::new_unique();
        let from = Pubkey::new_unique();
        let payload = b"Hello, agent!".to_vec();

        let instruction = post_message(&to, &from, payload.clone());

        assert_eq!(instruction.program_id, MULTI_AGENT_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 3);
    }

    #[test]
    fn test_coordinator() {
        let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let mut coordinator = MultiAgentCoordinator::new(client);

        let agent1 = Pubkey::new_unique();
        let agent2 = Pubkey::new_unique();

        coordinator.register_agent(&agent1);
        coordinator.register_agent(&agent2);

        assert_eq!(coordinator.agents().len(), 2);
        assert!(coordinator.get_inbox(&agent1).is_some());
    }
}
