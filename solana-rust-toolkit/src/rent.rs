//! Rent recovery utilities for reclaiming SOL from unused accounts.

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use spl_token::state::Account as TokenAccount;
use std::sync::Arc;

use crate::{Result, ToolkitError};

/// Configuration for rent recovery operations.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Owner of the accounts to recover rent from.
    pub owner: Pubkey,
    /// Minimum balance in lamports to consider for recovery.
    pub min_balance_lamports: u64,
    /// If true, only simulate and report without executing.
    pub dry_run: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            min_balance_lamports: 5000,
            dry_run: true,
        }
    }
}

/// Result of a rent recovery operation.
#[derive(Debug, Default)]
pub struct RecoveryResult {
    /// Total lamports recovered.
    pub total_recovered: u64,
    /// Number of accounts closed.
    pub accounts_closed: u32,
    /// Transaction signatures.
    pub signatures: Vec<Signature>,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// Information about a reclaimable account.
#[derive(Debug, Clone)]
pub struct ReclaimableAccount {
    /// Account public key.
    pub pubkey: Pubkey,
    /// Current balance in lamports.
    pub lamports: u64,
    /// Size of account data.
    pub data_len: usize,
    /// Account type.
    pub account_type: AccountType,
}

/// Type of account for recovery purposes.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    /// Empty data account.
    Empty,
    /// Token account with zero balance.
    TokenEmpty,
    /// Token account with non-zero balance.
    TokenWithBalance,
    /// System account.
    System,
    /// Unknown/other.
    Other,
}

/// Handles rent recovery operations.
pub struct RentRecovery {
    rpc: Arc<RpcClient>,
}

impl RentRecovery {
    /// Create a new RentRecovery instance.
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );
        Ok(Self { rpc: Arc::new(rpc) })
    }

    /// Create with an existing RPC client.
    pub fn with_client(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Find all accounts that can have rent reclaimed.
    pub async fn find_reclaimable_accounts(
        &self,
        config: &RecoveryConfig,
    ) -> Result<Vec<ReclaimableAccount>> {
        let mut reclaimable = Vec::new();

        // Get all token accounts owned by the user
        let token_accounts = self.get_token_accounts(&config.owner).await?;

        for (pubkey, account) in token_accounts {
            if account.lamports < config.min_balance_lamports {
                continue;
            }

            // Try to parse as token account
            if account.data.len() == TokenAccount::LEN {
                if let Ok(token_account) = self.parse_token_account(&account.data) {
                    let account_type = if token_account.amount == 0 {
                        AccountType::TokenEmpty
                    } else {
                        AccountType::TokenWithBalance
                    };

                    reclaimable.push(ReclaimableAccount {
                        pubkey,
                        lamports: account.lamports,
                        data_len: account.data.len(),
                        account_type,
                    });
                }
            } else if account.data.is_empty() {
                reclaimable.push(ReclaimableAccount {
                    pubkey,
                    lamports: account.lamports,
                    data_len: 0,
                    account_type: AccountType::Empty,
                });
            }
        }

        Ok(reclaimable)
    }

    /// Recover rent from reclaimable accounts.
    pub async fn recover_rent(
        &self,
        config: &RecoveryConfig,
        payer: &Keypair,
    ) -> Result<RecoveryResult> {
        let accounts = self.find_reclaimable_accounts(config).await?;
        let mut result = RecoveryResult::default();

        if config.dry_run {
            for account in &accounts {
                if account.account_type == AccountType::TokenEmpty {
                    result.total_recovered += account.lamports;
                    result.accounts_closed += 1;
                }
            }
            return Ok(result);
        }

        // Process empty token accounts
        let empty_token_accounts: Vec<_> = accounts
            .iter()
            .filter(|a| a.account_type == AccountType::TokenEmpty)
            .collect();

        // Batch close instructions (max ~10 per tx to stay under compute limit)
        for chunk in empty_token_accounts.chunks(10) {
            let mut instructions = Vec::new();

            for account in chunk {
                let close_ix = spl_token::instruction::close_account(
                    &spl_token::id(),
                    &account.pubkey,
                    &config.owner,
                    &config.owner,
                    &[],
                )?;
                instructions.push(close_ix);
                result.total_recovered += account.lamports;
                result.accounts_closed += 1;
            }

            match self.send_transaction(&instructions, payer).await {
                Ok(sig) => result.signatures.push(sig),
                Err(e) => result.errors.push(e.to_string()),
            }
        }

        Ok(result)
    }

    /// Close all empty token accounts for an owner.
    pub async fn close_empty_token_accounts(
        &self,
        owner: &Pubkey,
        payer: &Keypair,
    ) -> Result<u64> {
        let config = RecoveryConfig {
            owner: *owner,
            min_balance_lamports: 0,
            dry_run: false,
        };

        let result = self.recover_rent(&config, payer).await?;
        Ok(result.accounts_closed as u64)
    }

    // Helper methods

    async fn get_token_accounts(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<(Pubkey, solana_sdk::account::Account)>> {
        use solana_client::rpc_filter::{Memcmp, RpcFilterType};
        use solana_client::rpc_config::RpcProgramAccountsConfig;
        use solana_account_decoder::UiAccountEncoding;

        let filters = vec![
            RpcFilterType::DataSize(TokenAccount::LEN as u64),
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(32, owner.as_ref())),
        ];

        let config = RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.rpc.get_program_accounts_with_config(
            &spl_token::id(),
            config,
        )?;

        Ok(accounts)
    }

    fn parse_token_account(&self, data: &[u8]) -> Result<TokenAccount> {
        use solana_program::program_pack::Pack;
        TokenAccount::unpack(data)
            .map_err(|_| ToolkitError::Deserialization("Failed to parse token account".into()))
    }

    async fn send_transaction(
        &self,
        instructions: &[Instruction],
        payer: &Keypair,
    ) -> Result<Signature> {
        let recent_blockhash = self.rpc.get_latest_blockhash()?;
        
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[payer],
            recent_blockhash,
        );

        let sig = self.rpc.send_and_confirm_transaction(&tx)?;
        Ok(sig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert!(config.dry_run);
        assert_eq!(config.min_balance_lamports, 5000);
    }
}
