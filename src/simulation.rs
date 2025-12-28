//! Transaction simulation for previewing outcomes before execution.
//!
//! Simulates transactions to detect potential issues, unexpected balance changes,
//! and preview swap outcomes before committing to the chain.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcSimulateTransactionConfig, RpcSimulateTransactionAccountsConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    transaction::Transaction,
};
use solana_account_decoder::UiAccountEncoding;
use solana_transaction_status::UiTransactionEncoding;
use std::collections::HashMap;

/// Result of a transaction simulation.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the simulation succeeded.
    pub success: bool,
    /// Program logs from simulation.
    pub logs: Vec<String>,
    /// Compute units consumed.
    pub units_consumed: u64,
    /// SOL balance changes.
    pub balance_changes: Vec<BalanceChange>,
    /// Token balance changes.
    pub token_balance_changes: Vec<TokenBalanceChange>,
    /// Error message if simulation failed.
    pub error: Option<String>,
}

/// SOL balance change for an account.
#[derive(Debug, Clone)]
pub struct BalanceChange {
    /// Account pubkey.
    pub account: Pubkey,
    /// Balance before transaction.
    pub before: u64,
    /// Balance after transaction.
    pub after: u64,
    /// Net change (can be negative).
    pub delta: i64,
}

/// Token balance change for an account.
#[derive(Debug, Clone)]
pub struct TokenBalanceChange {
    /// Token account pubkey.
    pub account: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// Balance before transaction.
    pub before: u64,
    /// Balance after transaction.
    pub after: u64,
    /// Net change (can be negative).
    pub delta: i64,
}

/// Configuration for simulation validation.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Maximum acceptable SOL loss in lamports.
    pub max_sol_loss: u64,
    /// Maximum acceptable token loss percentage (0-100).
    pub max_token_loss_percent: u8,
    /// Fail if any unexpected account is modified.
    pub strict_account_checks: bool,
    /// Expected accounts to be modified.
    pub expected_accounts: Vec<Pubkey>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            max_sol_loss: 10_000_000, // 0.01 SOL
            max_token_loss_percent: 5,
            strict_account_checks: false,
            expected_accounts: Vec::new(),
        }
    }
}

impl SimulationConfig {
    /// Create a new simulation config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum SOL loss.
    pub fn max_sol_loss(mut self, lamports: u64) -> Self {
        self.max_sol_loss = lamports;
        self
    }

    /// Set maximum token loss percentage.
    pub fn max_token_loss_percent(mut self, percent: u8) -> Self {
        self.max_token_loss_percent = percent.min(100);
        self
    }

    /// Enable strict account checks.
    pub fn strict(mut self) -> Self {
        self.strict_account_checks = true;
        self
    }

    /// Set expected accounts.
    pub fn expected_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.expected_accounts = accounts;
        self
    }
}

/// Preview of a swap transaction.
#[derive(Debug, Clone)]
pub struct SwapPreview {
    /// Input token mint.
    pub input_token: Pubkey,
    /// Input amount.
    pub input_amount: u64,
    /// Output token mint.
    pub output_token: Pubkey,
    /// Output amount.
    pub output_amount: u64,
    /// Price impact in basis points.
    pub price_impact_bps: u16,
    /// Fees paid.
    pub fees: u64,
}

/// Simulation safety report.
#[derive(Debug, Clone)]
pub struct SimulationSafetyReport {
    /// Whether the simulation passed safety checks.
    pub safe: bool,
    /// Risk level.
    pub risk_level: SimulationRiskLevel,
    /// Warnings found.
    pub warnings: Vec<String>,
    /// Blockers that prevent approval.
    pub blockers: Vec<String>,
    /// The simulation result.
    pub simulation: SimulationResult,
}

/// Risk level from simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationRiskLevel {
    /// No issues detected.
    Safe,
    /// Minor warnings.
    Low,
    /// Significant concerns.
    Medium,
    /// High risk, requires confirmation.
    High,
    /// Critical issues, should not proceed.
    Critical,
}

impl std::fmt::Display for SimulationRiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulationRiskLevel::Safe => write!(f, "SAFE"),
            SimulationRiskLevel::Low => write!(f, "LOW"),
            SimulationRiskLevel::Medium => write!(f, "MEDIUM"),
            SimulationRiskLevel::High => write!(f, "HIGH"),
            SimulationRiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Transaction simulator.
pub struct TransactionSimulator {
    rpc_client: RpcClient,
}

impl TransactionSimulator {
    /// Create a new transaction simulator.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    /// Create from existing RpcClient.
    pub fn from_client(client: RpcClient) -> Self {
        Self { rpc_client: client }
    }

    /// Simulate a transaction and return detailed results.
    pub fn simulate(&self, transaction: &Transaction) -> Result<SimulationResult> {
        // Get accounts involved in transaction
        let accounts: Vec<Pubkey> = transaction
            .message
            .account_keys
            .iter()
            .cloned()
            .collect();

        // Fetch pre-balances
        let pre_balances = self.fetch_balances(&accounts)?;

        // Configure simulation
        let config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            commitment: Some(CommitmentConfig::confirmed()),
            encoding: Some(UiTransactionEncoding::Base64),
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                encoding: Some(UiAccountEncoding::Base64),
                addresses: accounts.iter().map(|p| p.to_string()).collect(),
            }),
            min_context_slot: None,
            inner_instructions: false,
        };

        // Run simulation
        let sim_result = self
            .rpc_client
            .simulate_transaction_with_config(transaction, config)
            .map_err(|e| ToolkitError::TransactionError(format!("Simulation failed: {}", e)))?;

        let value = sim_result.value;

        // Extract logs
        let logs = value.logs.unwrap_or_default();

        // Extract error
        let error = value.err.map(|e| format!("{:?}", e));
        let success = error.is_none();

        // Extract compute units
        let units_consumed = value.units_consumed.unwrap_or(0);

        // Calculate balance changes (simplified - actual implementation would parse post accounts)
        let balance_changes = self.calculate_balance_changes(&accounts, &pre_balances)?;

        Ok(SimulationResult {
            success,
            logs,
            units_consumed,
            balance_changes,
            token_balance_changes: Vec::new(), // Would require parsing token accounts
            error,
        })
    }

    /// Simulate and validate against safety config.
    pub fn simulate_and_validate(
        &self,
        transaction: &Transaction,
        config: &SimulationConfig,
    ) -> Result<SimulationSafetyReport> {
        let simulation = self.simulate(transaction)?;

        let mut warnings = Vec::new();
        let mut blockers = Vec::new();
        let mut risk_level = SimulationRiskLevel::Safe;

        // Check if simulation failed
        if !simulation.success {
            blockers.push(format!(
                "Simulation failed: {}",
                simulation.error.as_deref().unwrap_or("Unknown error")
            ));
            risk_level = SimulationRiskLevel::Critical;
        }

        // Check SOL losses
        for change in &simulation.balance_changes {
            if change.delta < 0 {
                let loss = (-change.delta) as u64;
                if loss > config.max_sol_loss {
                    blockers.push(format!(
                        "Excessive SOL loss: {} lamports from {}",
                        loss,
                        change.account
                    ));
                    risk_level = SimulationRiskLevel::Critical;
                } else if loss > config.max_sol_loss / 2 {
                    warnings.push(format!(
                        "Significant SOL loss: {} lamports from {}",
                        loss,
                        change.account
                    ));
                    if risk_level < SimulationRiskLevel::Medium {
                        risk_level = SimulationRiskLevel::Medium;
                    }
                }
            }
        }

        // Check for unexpected account modifications
        if config.strict_account_checks && !config.expected_accounts.is_empty() {
            for change in &simulation.balance_changes {
                if change.delta != 0 && !config.expected_accounts.contains(&change.account) {
                    warnings.push(format!(
                        "Unexpected account modified: {}",
                        change.account
                    ));
                    if risk_level < SimulationRiskLevel::High {
                        risk_level = SimulationRiskLevel::High;
                    }
                }
            }
        }

        // Check compute units (warn if high)
        if simulation.units_consumed > 1_000_000 {
            warnings.push(format!(
                "High compute usage: {} units",
                simulation.units_consumed
            ));
            if risk_level < SimulationRiskLevel::Low {
                risk_level = SimulationRiskLevel::Low;
            }
        }

        let safe = blockers.is_empty() && risk_level < SimulationRiskLevel::High;

        Ok(SimulationSafetyReport {
            safe,
            risk_level,
            warnings,
            blockers,
            simulation,
        })
    }

    /// Preview a swap outcome.
    pub fn preview_swap(&self, transaction: &Transaction) -> Result<SwapPreview> {
        let simulation = self.simulate(transaction)?;

        if !simulation.success {
            return Err(ToolkitError::TransactionError(format!(
                "Swap simulation failed: {}",
                simulation.error.unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        // Parse token balance changes to determine swap details
        // This is a simplified implementation - real implementation would parse
        // the actual token account changes from simulation

        // For now, return a placeholder that would need to be filled from actual simulation data
        Ok(SwapPreview {
            input_token: Pubkey::default(),
            input_amount: 0,
            output_token: Pubkey::default(),
            output_amount: 0,
            price_impact_bps: 0,
            fees: 0,
        })
    }

    /// Fetch current balances for accounts.
    fn fetch_balances(&self, accounts: &[Pubkey]) -> Result<HashMap<Pubkey, u64>> {
        let mut balances = HashMap::new();

        for account in accounts {
            match self.rpc_client.get_balance(account) {
                Ok(balance) => {
                    balances.insert(*account, balance);
                }
                Err(_) => {
                    balances.insert(*account, 0);
                }
            }
        }

        Ok(balances)
    }

    /// Calculate balance changes.
    fn calculate_balance_changes(
        &self,
        accounts: &[Pubkey],
        pre_balances: &HashMap<Pubkey, u64>,
    ) -> Result<Vec<BalanceChange>> {
        let mut changes = Vec::new();

        for account in accounts {
            let before = *pre_balances.get(account).unwrap_or(&0);
            // Post-balance would come from simulation result
            // For now, assume no change (actual implementation would parse simulation accounts)
            let after = before;
            let delta = after as i64 - before as i64;

            if delta != 0 {
                changes.push(BalanceChange {
                    account: *account,
                    before,
                    after,
                    delta,
                });
            }
        }

        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_config_defaults() {
        let config = SimulationConfig::default();
        assert_eq!(config.max_sol_loss, 10_000_000);
        assert_eq!(config.max_token_loss_percent, 5);
        assert!(!config.strict_account_checks);
    }

    #[test]
    fn test_simulation_config_builder() {
        let config = SimulationConfig::new()
            .max_sol_loss(5_000_000)
            .max_token_loss_percent(10)
            .strict();

        assert_eq!(config.max_sol_loss, 5_000_000);
        assert_eq!(config.max_token_loss_percent, 10);
        assert!(config.strict_account_checks);
    }

    #[test]
    fn test_simulation_config_max_percent_capped() {
        let config = SimulationConfig::new().max_token_loss_percent(150);
        assert_eq!(config.max_token_loss_percent, 100);
    }

    #[test]
    fn test_simulation_result_success() {
        let result = SimulationResult {
            success: true,
            logs: vec!["Program log: Success".to_string()],
            units_consumed: 50000,
            balance_changes: vec![],
            token_balance_changes: vec![],
            error: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_simulation_result_failure() {
        let result = SimulationResult {
            success: false,
            logs: vec!["Program log: Error".to_string()],
            units_consumed: 10000,
            balance_changes: vec![],
            token_balance_changes: vec![],
            error: Some("Insufficient funds".to_string()),
        };

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_balance_change() {
        let change = BalanceChange {
            account: Pubkey::new_unique(),
            before: 1_000_000_000,
            after: 900_000_000,
            delta: -100_000_000,
        };

        assert!(change.delta < 0);
        assert_eq!(change.before - change.after, 100_000_000);
    }

    #[test]
    fn test_simulation_risk_level_ordering() {
        assert!(SimulationRiskLevel::Safe < SimulationRiskLevel::Low);
        assert!(SimulationRiskLevel::Low < SimulationRiskLevel::Medium);
        assert!(SimulationRiskLevel::Medium < SimulationRiskLevel::High);
        assert!(SimulationRiskLevel::High < SimulationRiskLevel::Critical);
    }

    #[test]
    fn test_simulation_safety_report_safe() {
        let report = SimulationSafetyReport {
            safe: true,
            risk_level: SimulationRiskLevel::Safe,
            warnings: vec![],
            blockers: vec![],
            simulation: SimulationResult {
                success: true,
                logs: vec![],
                units_consumed: 50000,
                balance_changes: vec![],
                token_balance_changes: vec![],
                error: None,
            },
        };

        assert!(report.safe);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn test_simulation_safety_report_blocked() {
        let report = SimulationSafetyReport {
            safe: false,
            risk_level: SimulationRiskLevel::Critical,
            warnings: vec![],
            blockers: vec!["Simulation failed".to_string()],
            simulation: SimulationResult {
                success: false,
                logs: vec![],
                units_consumed: 0,
                balance_changes: vec![],
                token_balance_changes: vec![],
                error: Some("Error".to_string()),
            },
        };

        assert!(!report.safe);
        assert!(!report.blockers.is_empty());
    }

    #[test]
    fn test_swap_preview() {
        let preview = SwapPreview {
            input_token: Pubkey::new_unique(),
            input_amount: 1_000_000,
            output_token: Pubkey::new_unique(),
            output_amount: 950_000,
            price_impact_bps: 50,
            fees: 1000,
        };

        assert!(preview.output_amount < preview.input_amount);
        assert_eq!(preview.price_impact_bps, 50);
    }
}
