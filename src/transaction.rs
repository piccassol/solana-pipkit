//! Transaction batching and building utilities.
//!
//! This module provides utilities for building, batching, and executing
//! Solana transactions efficiently with automatic size management.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::{Result, ToolkitError};

/// Maximum transaction size in bytes (1232 bytes for legacy transactions).
pub const MAX_TRANSACTION_SIZE: usize = 1232;

/// Maximum number of accounts per transaction (64 for legacy).
pub const MAX_ACCOUNTS_PER_TX: usize = 64;

/// Default compute units per transaction.
pub const DEFAULT_COMPUTE_UNITS: u32 = 200_000;

/// Configuration for transaction execution.
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    /// Compute unit limit for the transaction.
    pub compute_units: Option<u32>,
    /// Priority fee in micro-lamports per compute unit.
    pub priority_fee_micro_lamports: Option<u64>,
    /// Whether to simulate before sending.
    pub simulate_before_send: bool,
    /// Whether to skip preflight checks.
    pub skip_preflight: bool,
    /// Number of retry attempts on failure.
    pub max_retries: u8,
    /// Commitment level for confirmation.
    pub commitment: CommitmentConfig,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            compute_units: None,
            priority_fee_micro_lamports: None,
            simulate_before_send: true,
            skip_preflight: false,
            max_retries: 3,
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

impl TransactionConfig {
    /// Create a config optimized for speed (skip simulation, preflight).
    pub fn fast() -> Self {
        Self {
            simulate_before_send: false,
            skip_preflight: true,
            max_retries: 1,
            ..Default::default()
        }
    }

    /// Create a config optimized for reliability.
    pub fn reliable() -> Self {
        Self {
            simulate_before_send: true,
            skip_preflight: false,
            max_retries: 5,
            commitment: CommitmentConfig::finalized(),
            ..Default::default()
        }
    }

    /// Set compute units.
    pub fn with_compute_units(mut self, units: u32) -> Self {
        self.compute_units = Some(units);
        self
    }

    /// Set priority fee in micro-lamports.
    pub fn with_priority_fee(mut self, micro_lamports: u64) -> Self {
        self.priority_fee_micro_lamports = Some(micro_lamports);
        self
    }
}

/// Result of a batch transaction execution.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Successful transaction signatures.
    pub successful: Vec<Signature>,
    /// Failed transactions with their errors.
    pub failed: Vec<(usize, String)>,
    /// Total instructions processed.
    pub instructions_processed: usize,
}

impl BatchResult {
    /// Check if all transactions succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.successful.len() + self.failed.len();
        if total == 0 {
            100.0
        } else {
            (self.successful.len() as f64 / total as f64) * 100.0
        }
    }
}

/// Transaction builder for constructing complex transactions.
#[derive(Default)]
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    signers: Vec<Pubkey>,
    config: TransactionConfig,
}

impl TransactionBuilder {
    /// Create a new transaction builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder with custom config.
    pub fn with_config(config: TransactionConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Add an instruction to the transaction.
    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add multiple instructions.
    pub fn add_instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }

    /// Add a signer pubkey (for account tracking).
    pub fn add_signer(mut self, signer: Pubkey) -> Self {
        if !self.signers.contains(&signer) {
            self.signers.push(signer);
        }
        self
    }

    /// Set compute units for this transaction.
    pub fn compute_units(mut self, units: u32) -> Self {
        self.config.compute_units = Some(units);
        self
    }

    /// Set priority fee for this transaction.
    pub fn priority_fee(mut self, micro_lamports: u64) -> Self {
        self.config.priority_fee_micro_lamports = Some(micro_lamports);
        self
    }

    /// Get the number of instructions currently in the builder.
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Estimate the number of unique accounts in the transaction.
    pub fn estimate_accounts(&self) -> usize {
        let mut accounts = std::collections::HashSet::new();
        for ix in &self.instructions {
            accounts.insert(ix.program_id);
            for meta in &ix.accounts {
                accounts.insert(meta.pubkey);
            }
        }
        accounts.len()
    }

    /// Check if adding more instructions might exceed transaction limits.
    pub fn might_exceed_limits(&self) -> bool {
        self.estimate_accounts() >= MAX_ACCOUNTS_PER_TX - 5
    }

    /// Build the final instructions with compute budget if configured.
    pub fn build_instructions(self) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Add compute budget instructions if configured
        if let Some(units) = self.config.compute_units {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(units));
        }

        if let Some(fee) = self.config.priority_fee_micro_lamports {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
        }

        instructions.extend(self.instructions);
        instructions
    }

    /// Build a transaction ready for signing.
    pub fn build(self, payer: &Pubkey, recent_blockhash: Hash) -> Transaction {
        let instructions = self.build_instructions();
        let message = Message::new(&instructions, Some(payer));
        Transaction::new_unsigned(message)
    }
}

/// Batch executor for processing multiple transactions.
pub struct BatchExecutor {
    client: RpcClient,
    config: TransactionConfig,
}

impl BatchExecutor {
    /// Create a new batch executor.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            config: TransactionConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(rpc_url: &str, config: TransactionConfig) -> Self {
        Self {
            client: RpcClient::new_with_commitment(rpc_url.to_string(), config.commitment),
            config,
        }
    }

    /// Execute a single transaction with the configured settings.
    pub async fn execute_transaction(
        &self,
        instructions: Vec<Instruction>,
        signers: &[&Keypair],
    ) -> Result<Signature> {
        if signers.is_empty() {
            return Err(ToolkitError::SigningError("No signers provided".to_string()));
        }

        let payer = signers[0];
        let mut all_instructions = Vec::new();

        // Add compute budget instructions if configured
        if let Some(units) = self.config.compute_units {
            all_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(units));
        }

        if let Some(fee) = self.config.priority_fee_micro_lamports {
            all_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
        }

        all_instructions.extend(instructions);

        let recent_blockhash = self.client.get_latest_blockhash().await?;
        let message = Message::new(&all_instructions, Some(&payer.pubkey()));
        let transaction = Transaction::new(signers, message, recent_blockhash);

        // Simulate if configured
        if self.config.simulate_before_send {
            let sim_result = self.client.simulate_transaction(&transaction).await?;
            if let Some(err) = sim_result.value.err {
                return Err(ToolkitError::TransactionError(format!(
                    "Simulation failed: {:?}",
                    err
                )));
            }
        }

        // Send with retries
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            match self.client.send_and_confirm_transaction(&transaction).await {
                Ok(sig) => return Ok(sig),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        Err(ToolkitError::TransactionError(format!(
            "Failed after {} retries: {:?}",
            self.config.max_retries,
            last_error
        )))
    }

    /// Split instructions into batches based on account limits.
    pub fn split_into_batches(&self, instructions: Vec<Instruction>, max_per_batch: usize) -> Vec<Vec<Instruction>> {
        let effective_max = max_per_batch.min(MAX_ACCOUNTS_PER_TX / 4); // Conservative estimate
        instructions.chunks(effective_max).map(|c| c.to_vec()).collect()
    }

    /// Execute multiple instruction batches sequentially.
    pub async fn execute_batches(
        &self,
        instruction_batches: Vec<Vec<Instruction>>,
        signers: &[&Keypair],
    ) -> Result<BatchResult> {
        let mut result = BatchResult {
            successful: Vec::new(),
            failed: Vec::new(),
            instructions_processed: 0,
        };

        for (batch_idx, instructions) in instruction_batches.into_iter().enumerate() {
            let ix_count = instructions.len();
            match self.execute_transaction(instructions, signers).await {
                Ok(sig) => {
                    result.successful.push(sig);
                    result.instructions_processed += ix_count;
                }
                Err(e) => {
                    result.failed.push((batch_idx, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Execute all instructions, automatically batching as needed.
    pub async fn execute_all(
        &self,
        instructions: Vec<Instruction>,
        signers: &[&Keypair],
        max_per_batch: usize,
    ) -> Result<BatchResult> {
        let batches = self.split_into_batches(instructions, max_per_batch);
        self.execute_batches(batches, signers).await
    }
}

/// Parallel batch executor for concurrent transaction processing.
pub struct ParallelBatchExecutor {
    client: RpcClient,
    config: TransactionConfig,
    max_concurrent: usize,
}

impl ParallelBatchExecutor {
    /// Create a new parallel batch executor.
    pub fn new(rpc_url: &str, max_concurrent: usize) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            config: TransactionConfig::default(),
            max_concurrent,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(rpc_url: &str, config: TransactionConfig, max_concurrent: usize) -> Self {
        Self {
            client: RpcClient::new_with_commitment(rpc_url.to_string(), config.commitment),
            config,
            max_concurrent,
        }
    }

    /// Execute multiple independent transactions in parallel.
    pub async fn execute_parallel(
        &self,
        transaction_instructions: Vec<Vec<Instruction>>,
        signers: &[&Keypair],
    ) -> Result<BatchResult> {
        use futures::stream::{self, StreamExt};

        if signers.is_empty() {
            return Err(ToolkitError::SigningError("No signers provided".to_string()));
        }

        let payer = signers[0];
        let recent_blockhash = self.client.get_latest_blockhash().await?;

        let transactions: Vec<_> = transaction_instructions
            .iter()
            .map(|instructions| {
                let mut all_instructions = Vec::new();

                if let Some(units) = self.config.compute_units {
                    all_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(units));
                }

                if let Some(fee) = self.config.priority_fee_micro_lamports {
                    all_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
                }

                all_instructions.extend(instructions.clone());

                let message = Message::new(&all_instructions, Some(&payer.pubkey()));
                Transaction::new(signers, message, recent_blockhash)
            })
            .collect();

        let results: Vec<_> = stream::iter(transactions.into_iter().enumerate())
            .map(|(idx, tx)| {
                let client = &self.client;
                async move {
                    match client.send_and_confirm_transaction(&tx).await {
                        Ok(sig) => (idx, Ok(sig)),
                        Err(e) => (idx, Err(e.to_string())),
                    }
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect()
            .await;

        let mut batch_result = BatchResult {
            successful: Vec::new(),
            failed: Vec::new(),
            instructions_processed: 0,
        };

        for (idx, result) in results {
            match result {
                Ok(sig) => {
                    batch_result.successful.push(sig);
                    batch_result.instructions_processed += transaction_instructions[idx].len();
                }
                Err(e) => {
                    batch_result.failed.push((idx, e));
                }
            }
        }

        Ok(batch_result)
    }
}

/// Estimate transaction size for a set of instructions.
pub fn estimate_transaction_size(instructions: &[Instruction], num_signers: usize) -> usize {
    let mut size = 0;

    // Signature size (64 bytes per signer)
    size += num_signers * 64;

    // Message header (3 bytes)
    size += 3;

    // Account keys
    let mut accounts = std::collections::HashSet::new();
    for ix in instructions {
        accounts.insert(ix.program_id);
        for meta in &ix.accounts {
            accounts.insert(meta.pubkey);
        }
    }
    size += accounts.len() * 32;

    // Recent blockhash
    size += 32;

    // Instruction count (compact-u16)
    size += 1;

    // Instructions
    for ix in instructions {
        size += 1; // Program ID index
        size += 1; // Account indices length
        size += ix.accounts.len(); // Account indices
        size += 2; // Data length (compact-u16)
        size += ix.data.len(); // Data
    }

    size
}

/// Check if a set of instructions will fit in a single transaction.
pub fn will_fit_in_transaction(instructions: &[Instruction], num_signers: usize) -> bool {
    estimate_transaction_size(instructions, num_signers) <= MAX_TRANSACTION_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::system_instruction;

    #[test]
    fn test_transaction_config_default() {
        let config = TransactionConfig::default();
        assert!(config.simulate_before_send);
        assert!(!config.skip_preflight);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_transaction_config_fast() {
        let config = TransactionConfig::fast();
        assert!(!config.simulate_before_send);
        assert!(config.skip_preflight);
    }

    #[test]
    fn test_transaction_builder() {
        let payer = Pubkey::new_unique();
        let to = Pubkey::new_unique();

        let builder = TransactionBuilder::new()
            .add_instruction(system_instruction::transfer(&payer, &to, 1000))
            .compute_units(100_000)
            .priority_fee(1000);

        assert_eq!(builder.instruction_count(), 1);
    }

    #[test]
    fn test_batch_result() {
        let result = BatchResult {
            successful: vec![Signature::default()],
            failed: vec![],
            instructions_processed: 5,
        };

        assert!(result.all_succeeded());
        assert_eq!(result.success_rate(), 100.0);
    }

    #[test]
    fn test_estimate_transaction_size() {
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let instructions = vec![system_instruction::transfer(&from, &to, 1000)];

        let size = estimate_transaction_size(&instructions, 1);
        assert!(size > 0);
        assert!(size < MAX_TRANSACTION_SIZE);
    }
}
