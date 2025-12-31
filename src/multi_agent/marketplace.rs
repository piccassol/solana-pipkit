//! Task marketplace for agent coordination.
//!
//! This module provides utilities for bidding on and executing tasks
//! in multi-agent systems, compatible with Anchor-based marketplaces.

use crate::{Result, ToolkitError};
use solana_sdk::{
    account::Account,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

/// Task marketplace program ID.
pub const TASK_MARKETPLACE_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TaskMark1111111111111111111111111111111");

/// Task account PDA seed.
pub const TASK_SEED: &[u8] = b"task";

/// Bid account PDA seed.
pub const BID_SEED: &[u8] = b"bid";

/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskStatus {
    /// Task is open for bidding.
    Open = 0,
    /// Task has been assigned.
    Assigned = 1,
    /// Task is in progress.
    InProgress = 2,
    /// Task completed successfully.
    Completed = 3,
    /// Task failed.
    Failed = 4,
    /// Task cancelled.
    Cancelled = 5,
}

impl TaskStatus {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Open),
            1 => Some(Self::Assigned),
            2 => Some(Self::InProgress),
            3 => Some(Self::Completed),
            4 => Some(Self::Failed),
            5 => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// On-chain task data.
#[derive(Debug, Clone)]
pub struct TaskAccount {
    /// Unique task ID.
    pub task_id: Hash,
    /// Task creator.
    pub creator: Pubkey,
    /// Assigned agent (if any).
    pub assigned_agent: Option<Pubkey>,
    /// Task description (hash).
    pub description: Hash,
    /// Reward amount in lamports.
    pub reward: u64,
    /// Task status.
    pub status: TaskStatus,
    /// Creation timestamp.
    pub created_at: i64,
    /// Completion deadline.
    pub deadline: Option<i64>,
    /// PDA bump.
    pub bump: u8,
}

/// Bid on a task.
#[derive(Debug, Clone)]
pub struct TaskBid {
    /// Task ID.
    pub task_id: Pubkey,
    /// Bidder agent.
    pub bidder: Pubkey,
    /// Bid amount (can be 0 for tasks with fixed reward).
    pub amount: u64,
    /// Optional proposal data.
    pub proposal: Vec<u8>,
    /// Bid timestamp.
    pub timestamp: i64,
}

impl TaskBid {
    /// Create a new task bid.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task's pubkey
    /// * `bidder` - The bidding agent's pubkey
    /// * `amount` - Bid amount (0 if accepting task at listed reward)
    ///
    /// # Returns
    ///
    /// A Solana instruction to place the bid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use solana_pipkit::multi_agent::TaskBid;
    /// use solana_sdk::pubkey::Pubkey;
    ///
    /// let task = Pubkey::new_unique();
    /// let bidder = Pubkey::new_unique();
    /// let instruction = TaskBid::new(task, bidder, 1_000_000);
    /// ```
    pub fn new(task_id: Pubkey, bidder: Pubkey, amount: u64) -> Instruction {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut data = vec![];
        data.push(0); // Instruction discriminator: PlaceBid
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());

        Instruction {
            program_id: TASK_MARKETPLACE_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(task_id, false),
                AccountMeta::new_readonly(bidder, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Create a bid with proposal data.
    pub fn with_proposal(task_id: Pubkey, bidder: Pubkey, amount: u64, proposal: Vec<u8>) -> Instruction {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut data = vec![];
        data.push(0); // Instruction discriminator: PlaceBid
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&(proposal.len() as u32).to_le_bytes());
        data.extend_from_slice(&proposal);

        Instruction {
            program_id: TASK_MARKETPLACE_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(task_id, false),
                AccountMeta::new_readonly(bidder, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }
}

/// Create a new task instruction.
#[derive(Debug, Clone)]
pub struct CreateTask {
    /// Task creator.
    pub creator: Pubkey,
    /// Task description hash.
    pub description: Hash,
    /// Reward amount.
    pub reward: u64,
    /// Deadline (optional).
    pub deadline: Option<i64>,
}

impl CreateTask {
    /// Create a new task instruction.
    ///
    /// # Arguments
    ///
    /// * `creator` - Task creator's pubkey
    /// * `description` - Hash of task description
    /// * `reward` - Reward amount in lamports
    /// * `deadline` - Optional deadline as Unix timestamp
    pub fn new(creator: Pubkey, description: Hash, reward: u64, deadline: Option<i64>) -> Instruction {
        let (task_pubkey, _) = Pubkey::find_program_address(
            &[TASK_SEED, description.as_ref()],
            &TASK_MARKETPLACE_PROGRAM_ID,
        );

        let mut data = vec![];
        data.push(1); // Instruction discriminator: CreateTask
        data.extend_from_slice(description.as_ref());
        data.extend_from_slice(&reward.to_le_bytes());

        if let Some(deadline_ts) = deadline {
            data.push(1);
            data.extend_from_slice(&deadline_ts.to_le_bytes());
        } else {
            data.push(0);
        }

        Instruction {
            program_id: TASK_MARKETPLACE_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(task_pubkey, false),
                AccountMeta::new_readonly(creator, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }
}

/// Accept a task instruction.
pub fn accept_task(task_id: Pubkey, assignee: Pubkey) -> Instruction {
    let mut data = vec![];
    data.push(2); // Instruction discriminator: AcceptTask

    Instruction {
        program_id: TASK_MARKETPLACE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(task_id, false),
            AccountMeta::new_readonly(assignee, true),
        ],
        data,
    }
}

/// Complete a task instruction.
pub fn complete_task(task_id: Pubkey, assignee: Pubkey, result: Vec<u8>) -> Instruction {
    let mut data = vec![];
    data.push(3); // Instruction discriminator: CompleteTask
    data.extend_from_slice(&(result.len() as u32).to_le_bytes());
    data.extend_from_slice(&result);

    Instruction {
        program_id: TASK_MARKETPLACE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(task_id, false),
            AccountMeta::new_readonly(assignee, true),
        ],
        data,
    }
}

/// Cancel a task instruction.
pub fn cancel_task(task_id: Pubkey, creator: Pubkey) -> Instruction {
    let mut data = vec![];
    data.push(4); // Instruction discriminator: CancelTask

    Instruction {
        program_id: TASK_MARKETPLACE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(task_id, false),
            AccountMeta::new_readonly(creator, true),
        ],
        data,
    }
}

/// Task marketplace for managing tasks and bids.
pub struct TaskMarketplace {
    client: solana_client::rpc_client::RpcClient,
}

impl TaskMarketplace {
    /// Create a new task marketplace client.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: solana_client::rpc_client::RpcClient::new(rpc_url.to_string()),
        }
    }

    /// Create a new task.
    pub fn create_task(
        &self,
        payer: &Keypair,
        description: Hash,
        reward: u64,
        deadline: Option<i64>,
    ) -> Result<Hash> {
        let instruction = CreateTask::new(payer.pubkey(), description, reward, deadline);
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Place a bid on a task.
    pub fn place_bid(
        &self,
        payer: &Keypair,
        task_id: Pubkey,
        amount: u64,
    ) -> Result<Hash> {
        let instruction = TaskBid::new(task_id, payer.pubkey(), amount);
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Accept a task.
    pub fn accept_task(
        &self,
        payer: &Keypair,
        task_id: Pubkey,
    ) -> Result<Hash> {
        let instruction = accept_task(task_id, payer.pubkey());
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Complete a task.
    pub fn complete_task(
        &self,
        payer: &Keypair,
        task_id: Pubkey,
        result: Vec<u8>,
    ) -> Result<Hash> {
        let instruction = complete_task(task_id, payer.pubkey(), result);
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Cancel a task.
    pub fn cancel_task(
        &self,
        payer: &Keypair,
        task_id: Pubkey,
    ) -> Result<Hash> {
        let instruction = cancel_task(task_id, payer.pubkey());
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let transaction = Transaction::new(
            &[payer],
            solana_sdk::message::Message::new(&[instruction], Some(&payer.pubkey())),
            recent_blockhash,
        );

        let signature = self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Get task account data.
    pub fn get_task(&self, task_id: &Pubkey) -> Result<TaskAccount> {
        let account = self.client
            .get_account(task_id)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        if account.data.len() < 32 + 32 + 1 + 32 + 8 + 1 + 8 {
            return Err(ToolkitError::InvalidAccountData("Task account data too short".to_string()));
        }

        let task_id = Hash::from(&account.data[0..32]);
        let creator = Pubkey::try_from(&account.data[32..64])
            .map_err(|_| ToolkitError::ParseError("Invalid creator pubkey".to_string()))?;

        let assigned_agent_offset = 64;
        let has_assigned = account.data[assigned_agent_offset] != 0;
        let assigned_agent = if has_assigned {
            Some(Pubkey::try_from(&account.data[assigned_agent_offset + 1..assigned_agent_offset + 33])
                .map_err(|_| ToolkitError::ParseError("Invalid assigned agent pubkey".to_string()))?)
        } else {
            None
        };

        let mut offset = if has_assigned {
            assigned_agent_offset + 33
        } else {
            assigned_agent_offset + 1
        };

        let description = Hash::from(&account.data[offset..offset + 32]);
        offset += 32;

        let reward = u64::from_le_bytes(account.data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let status = TaskStatus::from_u8(account.data[offset])
            .ok_or_else(|| ToolkitError::ParseError("Invalid task status".to_string()))?;
        offset += 1;

        let created_at = i64::from_le_bytes(account.data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let deadline = if account.data[offset] != 0 {
            Some(i64::from_le_bytes(account.data[offset + 1..offset + 9].try_into().unwrap()))
        } else {
            None
        };

        let bump = account.data[account.data.len() - 1];

        Ok(TaskAccount {
            task_id,
            creator,
            assigned_agent,
            description,
            reward,
            status,
            created_at,
            deadline,
            bump,
        })
    }

    /// Find tasks by creator.
    pub fn find_tasks_by_creator(&self, creator: &Pubkey) -> Result<Vec<Pubkey>> {
        let signatures = self.client
            .get_signatures_for_address(creator)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let mut task_ids = Vec::new();

        for sig_info in signatures {
            if let Ok(tx) = self.client.get_transaction(&sig_info.signature) {
                if let Some(meta) = &tx.transaction.meta {
                    if meta.err.is_none() {
                        if let Ok(task) = self.parse_task_from_transaction(&tx) {
                            if task.creator == *creator {
                                task_ids.push(task.task_id);
                            }
                        }
                    }
                }
            }
        }

        Ok(task_ids)
    }

    fn parse_task_from_transaction(
        &self,
        _tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TaskAccount> {
        Err(ToolkitError::ParseError("Task transaction parsing not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::from_u8(0), Some(TaskStatus::Open));
        assert_eq!(TaskStatus::from_u8(3), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_u8(99), None);
    }

    #[test]
    fn test_task_bid_creation() {
        let task = Pubkey::new_unique();
        let bidder = Pubkey::new_unique();
        let instruction = TaskBid::new(task, bidder, 1_000_000);

        assert_eq!(instruction.program_id, TASK_MARKETPLACE_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 3);
        assert_eq!(instruction.accounts[0].pubkey, task);
        assert_eq!(instruction.accounts[1].pubkey, bidder);
    }

    #[test]
    fn test_create_task_instruction() {
        let creator = Pubkey::new_unique();
        let description = Hash::new_unique();
        let reward = 5_000_000;
        let deadline = Some(1234567890);

        let instruction = CreateTask::new(creator, description, reward, deadline);

        assert_eq!(instruction.program_id, TASK_MARKETPLACE_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 3);
    }

    #[test]
    fn test_accept_task_instruction() {
        let task = Pubkey::new_unique();
        let assignee = Pubkey::new_unique();
        let instruction = accept_task(task, assignee);

        assert_eq!(instruction.program_id, TASK_MARKETPLACE_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 2);
    }

    #[test]
    fn test_complete_task_instruction() {
        let task = Pubkey::new_unique();
        let assignee = Pubkey::new_unique();
        let result = vec![1, 2, 3, 4];
        let instruction = complete_task(task, assignee, result);

        assert_eq!(instruction.program_id, TASK_MARKETPLACE_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 2);
    }

    #[test]
    fn test_cancel_task_instruction() {
        let task = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let instruction = cancel_task(task, creator);

        assert_eq!(instruction.program_id, TASK_MARKETPLACE_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 2);
    }
}
