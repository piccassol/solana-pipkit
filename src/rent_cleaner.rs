//! Rent Cleaner - Reclaim SOL from empty or unused accounts.
//!
//! This module provides utilities to scan your wallet for accounts
//! that can be closed to recover rent-exempt SOL. Includes advanced
//! recovery strategies for different account types and batched operations.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use spl_token::instruction as token_instruction;
use std::collections::HashMap;

use crate::{Result, ToolkitError};

/// Configuration for rent cleaning operations.
#[derive(Debug, Clone)]
pub struct RentCleanerConfig {
    /// Minimum lamports to consider an account "empty"
    pub min_balance_threshold: u64,
    /// Whether to close empty token accounts
    pub close_token_accounts: bool,
    /// Whether to close empty system accounts
    pub close_system_accounts: bool,
    /// Dry run mode (don't actually close accounts)
    pub dry_run: bool,
}

impl Default for RentCleanerConfig {
    fn default() -> Self {
        Self {
            min_balance_threshold: 0,
            close_token_accounts: true,
            close_system_accounts: true,
            dry_run: false,
        }
    }
}

/// Account information for potential cleanup.
#[derive(Debug, Clone)]
pub struct CleanableAccount {
    pub address: Pubkey,
    pub lamports: u64,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    TokenAccount,
    SystemAccount,
    Unknown,
}

/// Rent cleaner for recovering SOL from empty accounts.
pub struct RentCleaner {
    client: RpcClient,
    payer: Keypair,
    config: RentCleanerConfig,
}

impl RentCleaner {
    /// Create a new RentCleaner instance.
    pub fn new(rpc_url: &str, payer: Keypair) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            payer,
            config: RentCleanerConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(rpc_url: &str, payer: Keypair, config: RentCleanerConfig) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            payer,
            config,
        }
    }

    /// Scan for empty token accounts owned by the payer.
    pub async fn find_empty_token_accounts(&self) -> Result<Vec<CleanableAccount>> {
        let owner = self.payer.pubkey();
        let token_program = spl_token::id();

        let accounts = self
            .client
            .get_token_accounts_by_owner(&owner, solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program))
            .await?;

        let mut cleanable = Vec::new();

        for keyed_account in accounts {
            let pubkey = keyed_account.pubkey.parse::<Pubkey>().map_err(|e| {
                ToolkitError::Custom(format!("Failed to parse pubkey: {}", e))
            })?;

            // Check if token account has zero balance
            if let Some(account) = keyed_account.account.decode::<solana_sdk::account::Account>() {
                // Parse token account data
                if account.data.len() >= 64 {
                    let amount = u64::from_le_bytes(
                        account.data[64..72].try_into().unwrap_or([0; 8])
                    );
                    
                    if amount == 0 {
                        cleanable.push(CleanableAccount {
                            address: pubkey,
                            lamports: account.lamports,
                            account_type: AccountType::TokenAccount,
                        });
                    }
                }
            }
        }

        Ok(cleanable)
    }

    /// Close empty token accounts and recover rent.
    pub async fn close_empty_token_accounts(&self) -> Result<u64> {
        let accounts = self.find_empty_token_accounts().await?;
        let mut total_recovered: u64 = 0;

        if self.config.dry_run {
            for account in &accounts {
                println!(
                    "[DRY RUN] Would close token account {} and recover {} lamports",
                    account.address, account.lamports
                );
                total_recovered += account.lamports;
            }
            return Ok(total_recovered);
        }

        for account in accounts {
            match self.close_token_account(&account.address).await {
                Ok(lamports) => {
                    total_recovered += lamports;
                    println!(
                        "Closed token account {} - recovered {} lamports",
                        account.address, lamports
                    );
                }
                Err(e) => {
                    eprintln!("Failed to close {}: {}", account.address, e);
                }
            }
        }

        Ok(total_recovered)
    }

    /// Close a single token account.
    async fn close_token_account(&self, token_account: &Pubkey) -> Result<u64> {
        let account_info = self.client.get_account(token_account).await?;
        let lamports = account_info.lamports;

        let instruction = token_instruction::close_account(
            &spl_token::id(),
            token_account,
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &[],
        )?;

        self.send_transaction(vec![instruction]).await?;

        Ok(lamports)
    }

    /// Send a transaction with the given instructions.
    async fn send_transaction(&self, instructions: Vec<Instruction>) -> Result<()> {
        let recent_blockhash = self.client.get_latest_blockhash().await?;
        
        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        let transaction = Transaction::new(&[&self.payer], message, recent_blockhash);

        self.client
            .send_and_confirm_transaction(&transaction)
            .await?;

        Ok(())
    }

    /// Get total recoverable lamports from empty accounts.
    pub async fn estimate_recoverable(&self) -> Result<u64> {
        let accounts = self.find_empty_token_accounts().await?;
        Ok(accounts.iter().map(|a| a.lamports).sum())
    }
}

/// Convenience function to clean all empty accounts.
pub async fn clean_all_empty_accounts(rpc_url: &str, payer: Keypair) -> Result<u64> {
    let cleaner = RentCleaner::new(rpc_url, payer);
    cleaner.close_empty_token_accounts().await
}

// ============================================================================
// Advanced Rent Recovery Strategies
// ============================================================================

/// Strategy for handling different account types during cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupStrategy {
    /// Close only completely empty token accounts (balance = 0).
    EmptyOnly,
    /// Close accounts below a dust threshold (configurable).
    BelowDustThreshold,
    /// Burn remaining tokens and close (for worthless tokens).
    BurnAndClose,
    /// Aggregate small balances before closing.
    AggregateAndClose,
}

/// Priority level for cleanup operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanupPriority {
    /// High value recovery (largest rent amounts first).
    HighValue,
    /// Quick wins (easiest to close first).
    QuickWins,
    /// By mint (group token accounts by their mint).
    ByMint,
    /// Oldest first (by rent epoch).
    OldestFirst,
}

/// Advanced configuration for rent recovery.
#[derive(Debug, Clone)]
pub struct AdvancedCleanupConfig {
    /// Base configuration.
    pub base: RentCleanerConfig,
    /// Cleanup strategy to use.
    pub strategy: CleanupStrategy,
    /// Priority ordering.
    pub priority: CleanupPriority,
    /// Dust threshold in token smallest units (for BelowDustThreshold).
    pub dust_threshold: u64,
    /// Maximum accounts to process per batch.
    pub batch_size: usize,
    /// Whether to skip accounts that fail.
    pub skip_failures: bool,
    /// Mints to exclude from cleanup.
    pub excluded_mints: Vec<Pubkey>,
    /// Only process these mints (if empty, process all).
    pub included_mints: Vec<Pubkey>,
}

impl Default for AdvancedCleanupConfig {
    fn default() -> Self {
        Self {
            base: RentCleanerConfig::default(),
            strategy: CleanupStrategy::EmptyOnly,
            priority: CleanupPriority::HighValue,
            dust_threshold: 1,
            batch_size: 10,
            skip_failures: true,
            excluded_mints: Vec::new(),
            included_mints: Vec::new(),
        }
    }
}

impl AdvancedCleanupConfig {
    /// Create a config for aggressive cleanup (burn worthless tokens).
    pub fn aggressive() -> Self {
        Self {
            strategy: CleanupStrategy::BurnAndClose,
            skip_failures: true,
            ..Default::default()
        }
    }

    /// Create a config for conservative cleanup (empty only, no burns).
    pub fn conservative() -> Self {
        Self {
            strategy: CleanupStrategy::EmptyOnly,
            skip_failures: false,
            ..Default::default()
        }
    }

    /// Set the dust threshold.
    pub fn with_dust_threshold(mut self, threshold: u64) -> Self {
        self.dust_threshold = threshold;
        self
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Exclude specific mints.
    pub fn exclude_mints(mut self, mints: Vec<Pubkey>) -> Self {
        self.excluded_mints = mints;
        self
    }

    /// Only include specific mints.
    pub fn include_mints(mut self, mints: Vec<Pubkey>) -> Self {
        self.included_mints = mints;
        self
    }
}

/// Result of an advanced cleanup operation.
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// Total lamports recovered.
    pub lamports_recovered: u64,
    /// Number of accounts closed.
    pub accounts_closed: usize,
    /// Accounts that failed to close.
    pub failed_accounts: Vec<(Pubkey, String)>,
    /// Tokens burned (mint -> amount).
    pub tokens_burned: HashMap<Pubkey, u64>,
    /// Transaction signatures.
    pub signatures: Vec<Signature>,
}

impl CleanupResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            lamports_recovered: 0,
            accounts_closed: 0,
            failed_accounts: Vec::new(),
            tokens_burned: HashMap::new(),
            signatures: Vec::new(),
        }
    }

    /// Check if the cleanup was fully successful.
    pub fn is_complete_success(&self) -> bool {
        self.failed_accounts.is_empty()
    }

    /// Get the SOL equivalent recovered (assuming 1 SOL = 1e9 lamports).
    pub fn sol_recovered(&self) -> f64 {
        self.lamports_recovered as f64 / 1_000_000_000.0
    }
}

impl Default for CleanupResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Extended cleanable account with additional metadata.
#[derive(Debug, Clone)]
pub struct ExtendedCleanableAccount {
    /// Base cleanable account info.
    pub base: CleanableAccount,
    /// Token mint (if token account).
    pub mint: Option<Pubkey>,
    /// Token balance (if token account).
    pub token_balance: u64,
    /// Whether this account can be burned and closed.
    pub can_burn: bool,
}

/// Advanced rent cleaner with multiple recovery strategies.
pub struct AdvancedRentCleaner {
    client: RpcClient,
    payer: Keypair,
    config: AdvancedCleanupConfig,
}

impl AdvancedRentCleaner {
    /// Create a new advanced rent cleaner.
    pub fn new(rpc_url: &str, payer: Keypair) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            payer,
            config: AdvancedCleanupConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(rpc_url: &str, payer: Keypair, config: AdvancedCleanupConfig) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            payer,
            config,
        }
    }

    /// Scan for all cleanable accounts with extended information.
    pub async fn scan_accounts(&self) -> Result<Vec<ExtendedCleanableAccount>> {
        let owner = self.payer.pubkey();
        let token_program = spl_token::id();

        let accounts = self
            .client
            .get_token_accounts_by_owner(
                &owner,
                solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program),
            )
            .await?;

        let mut cleanable = Vec::new();

        for keyed_account in accounts {
            let pubkey = keyed_account.pubkey.parse::<Pubkey>().map_err(|e| {
                ToolkitError::Custom(format!("Failed to parse pubkey: {}", e))
            })?;

            if let Some(account) = keyed_account.account.decode::<solana_sdk::account::Account>() {
                if account.data.len() >= 72 {
                    // Parse token account data
                    let mint = Pubkey::try_from(&account.data[0..32]).ok();
                    let token_balance = u64::from_le_bytes(
                        account.data[64..72].try_into().unwrap_or([0; 8]),
                    );

                    // Check if we should include this account
                    if !self.should_include_account(mint.as_ref(), token_balance) {
                        continue;
                    }

                    let can_close = match self.config.strategy {
                        CleanupStrategy::EmptyOnly => token_balance == 0,
                        CleanupStrategy::BelowDustThreshold => {
                            token_balance <= self.config.dust_threshold
                        }
                        CleanupStrategy::BurnAndClose => true,
                        CleanupStrategy::AggregateAndClose => true,
                    };

                    if can_close {
                        cleanable.push(ExtendedCleanableAccount {
                            base: CleanableAccount {
                                address: pubkey,
                                lamports: account.lamports,
                                account_type: AccountType::TokenAccount,
                            },
                            mint,
                            token_balance,
                            can_burn: token_balance > 0,
                        });
                    }
                }
            }
        }

        // Sort by priority
        self.sort_by_priority(&mut cleanable);

        Ok(cleanable)
    }

    /// Check if an account should be included based on config.
    fn should_include_account(&self, mint: Option<&Pubkey>, _balance: u64) -> bool {
        if let Some(mint) = mint {
            // Check excluded mints
            if self.config.excluded_mints.contains(mint) {
                return false;
            }

            // Check included mints (if specified)
            if !self.config.included_mints.is_empty()
                && !self.config.included_mints.contains(mint)
            {
                return false;
            }
        }

        true
    }

    /// Sort accounts by priority.
    fn sort_by_priority(&self, accounts: &mut [ExtendedCleanableAccount]) {
        match self.config.priority {
            CleanupPriority::HighValue => {
                accounts.sort_by(|a, b| b.base.lamports.cmp(&a.base.lamports));
            }
            CleanupPriority::QuickWins => {
                // Empty accounts first, then by lamports
                accounts.sort_by(|a, b| {
                    let a_empty = a.token_balance == 0;
                    let b_empty = b.token_balance == 0;
                    b_empty.cmp(&a_empty).then(b.base.lamports.cmp(&a.base.lamports))
                });
            }
            CleanupPriority::ByMint => {
                accounts.sort_by(|a, b| {
                    a.mint.map(|m| m.to_string()).cmp(&b.mint.map(|m| m.to_string()))
                });
            }
            CleanupPriority::OldestFirst => {
                // Without rent epoch data, just sort by address for consistency
                accounts.sort_by(|a, b| a.base.address.cmp(&b.base.address));
            }
        }
    }

    /// Execute the cleanup with the configured strategy.
    pub async fn execute_cleanup(&self) -> Result<CleanupResult> {
        let accounts = self.scan_accounts().await?;
        let mut result = CleanupResult::new();

        if self.config.base.dry_run {
            for account in &accounts {
                println!(
                    "[DRY RUN] Would close {} (mint: {:?}, balance: {}, rent: {} lamports)",
                    account.base.address,
                    account.mint,
                    account.token_balance,
                    account.base.lamports
                );
                result.lamports_recovered += account.base.lamports;
                result.accounts_closed += 1;
            }
            return Ok(result);
        }

        // Process in batches
        for batch in accounts.chunks(self.config.batch_size) {
            let batch_result = self.process_batch(batch).await;
            match batch_result {
                Ok(sig) => {
                    for account in batch {
                        result.lamports_recovered += account.base.lamports;
                        result.accounts_closed += 1;
                        if account.token_balance > 0 {
                            if let Some(mint) = account.mint {
                                *result.tokens_burned.entry(mint).or_insert(0) +=
                                    account.token_balance;
                            }
                        }
                    }
                    result.signatures.push(sig);
                }
                Err(e) => {
                    if !self.config.skip_failures {
                        return Err(e);
                    }
                    // Process individually on batch failure
                    for account in batch {
                        match self.close_single_account(account).await {
                            Ok(sig) => {
                                result.lamports_recovered += account.base.lamports;
                                result.accounts_closed += 1;
                                if account.token_balance > 0 {
                                    if let Some(mint) = account.mint {
                                        *result.tokens_burned.entry(mint).or_insert(0) +=
                                            account.token_balance;
                                    }
                                }
                                result.signatures.push(sig);
                            }
                            Err(e) => {
                                result
                                    .failed_accounts
                                    .push((account.base.address, e.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Process a batch of accounts.
    async fn process_batch(&self, accounts: &[ExtendedCleanableAccount]) -> Result<Signature> {
        let mut instructions = Vec::new();
        let payer_pubkey = self.payer.pubkey();

        for account in accounts {
            // Burn tokens if needed
            if account.token_balance > 0 && self.config.strategy == CleanupStrategy::BurnAndClose {
                if let Some(mint) = account.mint {
                    instructions.push(token_instruction::burn(
                        &spl_token::id(),
                        &account.base.address,
                        &mint,
                        &payer_pubkey,
                        &[],
                        account.token_balance,
                    )?);
                }
            }

            // Close the account
            instructions.push(token_instruction::close_account(
                &spl_token::id(),
                &account.base.address,
                &payer_pubkey,
                &payer_pubkey,
                &[],
            )?);
        }

        self.send_transaction(instructions).await
    }

    /// Close a single account.
    async fn close_single_account(
        &self,
        account: &ExtendedCleanableAccount,
    ) -> Result<Signature> {
        let mut instructions = Vec::new();
        let payer_pubkey = self.payer.pubkey();

        // Burn tokens if needed
        if account.token_balance > 0 && self.config.strategy == CleanupStrategy::BurnAndClose {
            if let Some(mint) = account.mint {
                instructions.push(token_instruction::burn(
                    &spl_token::id(),
                    &account.base.address,
                    &mint,
                    &payer_pubkey,
                    &[],
                    account.token_balance,
                )?);
            }
        }

        // Close the account
        instructions.push(token_instruction::close_account(
            &spl_token::id(),
            &account.base.address,
            &payer_pubkey,
            &payer_pubkey,
            &[],
        )?);

        self.send_transaction(instructions).await
    }

    /// Send a transaction with the given instructions.
    async fn send_transaction(&self, instructions: Vec<Instruction>) -> Result<Signature> {
        let recent_blockhash = self.client.get_latest_blockhash().await?;
        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        let transaction = Transaction::new(&[&self.payer], message, recent_blockhash);

        self.client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))
    }

    /// Estimate total recoverable lamports.
    pub async fn estimate_recovery(&self) -> Result<u64> {
        let accounts = self.scan_accounts().await?;
        Ok(accounts.iter().map(|a| a.base.lamports).sum())
    }

    /// Get detailed breakdown of recoverable accounts.
    pub async fn get_recovery_breakdown(&self) -> Result<RecoveryBreakdown> {
        let accounts = self.scan_accounts().await?;

        let mut breakdown = RecoveryBreakdown {
            total_accounts: accounts.len(),
            total_lamports: 0,
            empty_accounts: 0,
            dust_accounts: 0,
            accounts_with_balance: 0,
            by_mint: HashMap::new(),
        };

        for account in accounts {
            breakdown.total_lamports += account.base.lamports;

            if account.token_balance == 0 {
                breakdown.empty_accounts += 1;
            } else if account.token_balance <= self.config.dust_threshold {
                breakdown.dust_accounts += 1;
            } else {
                breakdown.accounts_with_balance += 1;
            }

            if let Some(mint) = account.mint {
                let entry = breakdown.by_mint.entry(mint).or_insert(MintBreakdown {
                    mint,
                    account_count: 0,
                    total_lamports: 0,
                    total_tokens: 0,
                });
                entry.account_count += 1;
                entry.total_lamports += account.base.lamports;
                entry.total_tokens += account.token_balance;
            }
        }

        Ok(breakdown)
    }
}

/// Breakdown of recoverable rent by category.
#[derive(Debug, Clone)]
pub struct RecoveryBreakdown {
    /// Total number of cleanable accounts.
    pub total_accounts: usize,
    /// Total lamports recoverable.
    pub total_lamports: u64,
    /// Number of empty accounts (0 balance).
    pub empty_accounts: usize,
    /// Number of dust accounts (below threshold).
    pub dust_accounts: usize,
    /// Number of accounts with significant balance.
    pub accounts_with_balance: usize,
    /// Breakdown by mint.
    pub by_mint: HashMap<Pubkey, MintBreakdown>,
}

impl RecoveryBreakdown {
    /// Get total SOL recoverable.
    pub fn sol_recoverable(&self) -> f64 {
        self.total_lamports as f64 / 1_000_000_000.0
    }
}

/// Breakdown for a specific mint.
#[derive(Debug, Clone)]
pub struct MintBreakdown {
    /// The mint address.
    pub mint: Pubkey,
    /// Number of accounts for this mint.
    pub account_count: usize,
    /// Total lamports in these accounts.
    pub total_lamports: u64,
    /// Total token balance across accounts.
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RentCleanerConfig::default();
        assert!(config.close_token_accounts);
        assert!(config.close_system_accounts);
        assert!(!config.dry_run);
    }
}
