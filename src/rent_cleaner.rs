//! Rent Cleaner - Reclaim SOL from empty or unused accounts.
//!
//! This module provides utilities to scan your wallet for accounts
//! that can be closed to recover rent-exempt SOL.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token::instruction as token_instruction;

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
