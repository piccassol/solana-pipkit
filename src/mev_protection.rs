//! MEV protection utilities.
//!
//! Provides tools to protect against MEV attacks including sandwich attacks,
//! priority fee recommendations, and Jito bundle support.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;

/// Jito tip accounts for MEV protection.
pub const JITO_TIP_ACCOUNTS: [&str; 8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4bVmkdzGtLq7nNKBDLBLgCK",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

/// Priority fee recommendation based on recent network activity.
#[derive(Debug, Clone)]
pub struct PriorityFeeRecommendation {
    /// 25th percentile fee.
    pub low: u64,
    /// 50th percentile fee.
    pub medium: u64,
    /// 75th percentile fee.
    pub high: u64,
    /// 95th percentile fee.
    pub very_high: u64,
}

impl Default for PriorityFeeRecommendation {
    fn default() -> Self {
        Self {
            low: 1_000,
            medium: 10_000,
            high: 100_000,
            very_high: 1_000_000,
        }
    }
}

/// Sandwich attack risk analysis.
#[derive(Debug, Clone)]
pub struct SandwichRisk {
    /// Whether the transaction is at risk.
    pub is_risky: bool,
    /// Risk score from 0-100.
    pub risk_score: u8,
    /// Reasons for the risk assessment.
    pub reasons: Vec<String>,
    /// Recommendations to mitigate risk.
    pub recommendations: Vec<String>,
}

impl SandwichRisk {
    /// Create a low-risk assessment.
    pub fn low_risk() -> Self {
        Self {
            is_risky: false,
            risk_score: 10,
            reasons: vec![],
            recommendations: vec![],
        }
    }

    /// Create a high-risk assessment.
    pub fn high_risk(reasons: Vec<String>, recommendations: Vec<String>) -> Self {
        Self {
            is_risky: true,
            risk_score: 80,
            reasons,
            recommendations,
        }
    }
}

/// Jito bundle for MEV-protected transactions.
#[derive(Debug, Clone)]
pub struct JitoBundle {
    /// Serialized transactions.
    pub transactions: Vec<Vec<u8>>,
    /// Tip account used.
    pub tip_account: Pubkey,
    /// Tip amount in lamports.
    pub tip_amount: u64,
}

/// MEV protection helper.
pub struct MevProtection {
    rpc_client: RpcClient,
    /// Minimum USD value for sandwich target.
    sandwich_threshold_usd: f64,
}

impl MevProtection {
    /// Create a new MEV protection helper.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            sandwich_threshold_usd: 1000.0,
        }
    }

    /// Create from existing RpcClient.
    pub fn from_client(client: RpcClient) -> Self {
        Self {
            rpc_client: client,
            sandwich_threshold_usd: 1000.0,
        }
    }

    /// Set the sandwich attack threshold in USD.
    pub fn with_sandwich_threshold(mut self, threshold_usd: f64) -> Self {
        self.sandwich_threshold_usd = threshold_usd;
        self
    }

    /// Get recommended priority fees based on recent blocks.
    pub fn get_priority_fee_recommendation(&self) -> Result<PriorityFeeRecommendation> {
        // Try to get recent prioritization fees
        match self.rpc_client.get_recent_prioritization_fees(&[]) {
            Ok(fees) => {
                if fees.is_empty() {
                    return Ok(PriorityFeeRecommendation::default());
                }

                let mut fee_values: Vec<u64> = fees
                    .iter()
                    .map(|f| f.prioritization_fee)
                    .filter(|f| *f > 0)
                    .collect();

                if fee_values.is_empty() {
                    return Ok(PriorityFeeRecommendation::default());
                }

                fee_values.sort();

                let len = fee_values.len();
                let low = fee_values[len / 4];
                let medium = fee_values[len / 2];
                let high = fee_values[len * 3 / 4];
                let very_high = fee_values[len * 95 / 100.max(1).min(len - 1)];

                Ok(PriorityFeeRecommendation {
                    low: low.max(1_000),
                    medium: medium.max(10_000),
                    high: high.max(100_000),
                    very_high: very_high.max(1_000_000),
                })
            }
            Err(_) => Ok(PriorityFeeRecommendation::default()),
        }
    }

    /// Analyze a transaction for sandwich attack vulnerability.
    pub fn analyze_sandwich_risk(&self, transaction: &Transaction) -> SandwichRisk {
        let mut reasons = Vec::new();
        let mut recommendations = Vec::new();
        let mut risk_score: u8 = 0;

        // Check for DEX program interactions
        let dex_programs = self.get_known_dex_programs();
        let has_dex_interaction = transaction.message.account_keys.iter().any(|key| {
            dex_programs.contains(key)
        });

        if has_dex_interaction {
            reasons.push("Transaction interacts with DEX program".to_string());
            risk_score += 30;
        }

        // Check instruction count (complex transactions are higher risk)
        let instruction_count = transaction.message.instructions.len();
        if instruction_count > 3 {
            reasons.push(format!(
                "Complex transaction with {} instructions",
                instruction_count
            ));
            risk_score += 10;
        }

        // Check for token program interactions (swaps involve tokens)
        let has_token_interaction = transaction.message.account_keys.iter().any(|key| {
            *key == spl_token::id()
        });

        if has_token_interaction && has_dex_interaction {
            reasons.push("Token swap detected".to_string());
            risk_score += 20;
        }

        // Add recommendations based on risk
        if risk_score > 30 {
            recommendations.push("Use Jito bundles for MEV protection".to_string());
            recommendations.push("Set appropriate slippage tolerance".to_string());
        }

        if risk_score > 50 {
            recommendations.push("Consider using a private RPC".to_string());
            recommendations.push("Add priority fee to front-run attackers".to_string());
        }

        let is_risky = risk_score >= 40;

        SandwichRisk {
            is_risky,
            risk_score: risk_score.min(100),
            reasons,
            recommendations,
        }
    }

    /// Check if a swap amount makes it a sandwich target.
    pub fn is_sandwich_target(&self, swap_amount_usd: f64) -> bool {
        swap_amount_usd >= self.sandwich_threshold_usd
    }

    /// Get a random Jito tip account.
    pub fn get_jito_tip_account(&self) -> Result<Pubkey> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let index = (timestamp as usize) % JITO_TIP_ACCOUNTS.len();

        Pubkey::from_str(JITO_TIP_ACCOUNTS[index]).map_err(|e| {
            ToolkitError::ParseError(format!("Invalid Jito tip account: {}", e))
        })
    }

    /// Create a Jito tip instruction.
    pub fn create_jito_tip_instruction(
        &self,
        payer: &Pubkey,
        tip_lamports: u64,
    ) -> Result<Instruction> {
        let tip_account = self.get_jito_tip_account()?;

        Ok(Instruction {
            program_id: system_program::id(),
            accounts: vec![
                AccountMeta::new(*payer, true),
                AccountMeta::new(tip_account, false),
            ],
            data: bincode::serialize(&solana_sdk::system_instruction::SystemInstruction::Transfer {
                lamports: tip_lamports,
            })
            .map_err(|e| ToolkitError::Custom(format!("Failed to serialize: {}", e)))?,
        })
    }

    /// Add a Jito tip instruction to a transaction.
    pub fn add_jito_tip(
        &self,
        instructions: &mut Vec<Instruction>,
        payer: &Pubkey,
        tip_lamports: u64,
    ) -> Result<Pubkey> {
        let tip_account = self.get_jito_tip_account()?;

        let tip_instruction = solana_sdk::system_instruction::transfer(
            payer,
            &tip_account,
            tip_lamports,
        );

        instructions.push(tip_instruction);

        Ok(tip_account)
    }

    /// Create a Jito bundle from transactions.
    pub fn create_jito_bundle(
        &self,
        transactions: Vec<Transaction>,
        tip_lamports: u64,
    ) -> Result<JitoBundle> {
        let tip_account = self.get_jito_tip_account()?;

        let serialized: Vec<Vec<u8>> = transactions
            .iter()
            .map(|tx| bincode::serialize(tx))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| ToolkitError::Custom(format!("Failed to serialize transaction: {}", e)))?;

        Ok(JitoBundle {
            transactions: serialized,
            tip_account,
            tip_amount: tip_lamports,
        })
    }

    /// Get recommended tip amount based on transaction value.
    pub fn get_recommended_tip(&self, transaction_value_usd: f64) -> u64 {
        // Tip 0.1% of transaction value, minimum 10k lamports
        let tip_usd = transaction_value_usd * 0.001;
        let tip_lamports = (tip_usd * 1_000_000_000.0 / 100.0) as u64; // Assuming $100/SOL

        tip_lamports.max(10_000).min(10_000_000) // 10k to 10M lamports
    }

    /// Get known DEX program IDs.
    fn get_known_dex_programs(&self) -> Vec<Pubkey> {
        let mut programs = Vec::new();

        // Jupiter
        if let Ok(p) = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            programs.push(p);
        }
        if let Ok(p) = Pubkey::from_str("JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB") {
            programs.push(p);
        }

        // Raydium
        if let Ok(p) = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            programs.push(p);
        }
        if let Ok(p) = Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") {
            programs.push(p);
        }

        // Orca
        if let Ok(p) = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc") {
            programs.push(p);
        }
        if let Ok(p) = Pubkey::from_str("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            programs.push(p);
        }

        // Openbook/Serum
        if let Ok(p) = Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX") {
            programs.push(p);
        }

        programs
    }
}

/// Calculate optimal slippage based on trade size and liquidity.
pub fn calculate_optimal_slippage(trade_size_usd: f64, pool_liquidity_usd: f64) -> u16 {
    if pool_liquidity_usd == 0.0 {
        return 500; // 5% default
    }

    let trade_ratio = trade_size_usd / pool_liquidity_usd;

    // Base slippage + impact-based slippage
    let base_slippage = 50; // 0.5%
    let impact_slippage = (trade_ratio * 10000.0) as u16; // 1% per 1% of pool

    (base_slippage + impact_slippage).min(1000) // Cap at 10%
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_fee_recommendation_default() {
        let rec = PriorityFeeRecommendation::default();
        assert_eq!(rec.low, 1_000);
        assert_eq!(rec.medium, 10_000);
        assert_eq!(rec.high, 100_000);
        assert_eq!(rec.very_high, 1_000_000);
    }

    #[test]
    fn test_sandwich_risk_low() {
        let risk = SandwichRisk::low_risk();
        assert!(!risk.is_risky);
        assert_eq!(risk.risk_score, 10);
    }

    #[test]
    fn test_sandwich_risk_high() {
        let reasons = vec!["DEX interaction".to_string()];
        let recommendations = vec!["Use Jito".to_string()];
        let risk = SandwichRisk::high_risk(reasons.clone(), recommendations.clone());

        assert!(risk.is_risky);
        assert_eq!(risk.risk_score, 80);
        assert_eq!(risk.reasons, reasons);
        assert_eq!(risk.recommendations, recommendations);
    }

    #[test]
    fn test_is_sandwich_target() {
        let mev = MevProtection::new("http://localhost:8899");

        assert!(mev.is_sandwich_target(1000.0));
        assert!(mev.is_sandwich_target(5000.0));
        assert!(!mev.is_sandwich_target(500.0));
    }

    #[test]
    fn test_is_sandwich_target_custom_threshold() {
        let mev = MevProtection::new("http://localhost:8899")
            .with_sandwich_threshold(500.0);

        assert!(mev.is_sandwich_target(500.0));
        assert!(mev.is_sandwich_target(600.0));
        assert!(!mev.is_sandwich_target(400.0));
    }

    #[test]
    fn test_get_jito_tip_account() {
        let mev = MevProtection::new("http://localhost:8899");
        let tip_account = mev.get_jito_tip_account();

        assert!(tip_account.is_ok());
        let account = tip_account.unwrap();

        // Should be one of the known tip accounts
        let valid_accounts: Vec<Pubkey> = JITO_TIP_ACCOUNTS
            .iter()
            .filter_map(|s| Pubkey::from_str(s).ok())
            .collect();

        assert!(valid_accounts.contains(&account));
    }

    #[test]
    fn test_jito_tip_accounts_valid() {
        for account_str in JITO_TIP_ACCOUNTS {
            let result = Pubkey::from_str(account_str);
            assert!(result.is_ok(), "Invalid Jito tip account: {}", account_str);
        }
    }

    #[test]
    fn test_get_recommended_tip() {
        let mev = MevProtection::new("http://localhost:8899");

        // Small trade
        let tip = mev.get_recommended_tip(100.0);
        assert!(tip >= 10_000); // Minimum

        // Large trade
        let tip = mev.get_recommended_tip(100_000.0);
        assert!(tip <= 10_000_000); // Maximum

        // Medium trade
        let tip = mev.get_recommended_tip(10_000.0);
        assert!(tip > 10_000);
        assert!(tip <= 10_000_000);
    }

    #[test]
    fn test_calculate_optimal_slippage() {
        // Small trade relative to pool
        let slippage = calculate_optimal_slippage(100.0, 1_000_000.0);
        assert!(slippage <= 100); // Should be low

        // Large trade relative to pool
        let slippage = calculate_optimal_slippage(100_000.0, 1_000_000.0);
        assert!(slippage >= 100); // Should be higher

        // Very large trade
        let slippage = calculate_optimal_slippage(500_000.0, 1_000_000.0);
        assert_eq!(slippage, 1000); // Should be capped at 10%

        // Zero liquidity
        let slippage = calculate_optimal_slippage(100.0, 0.0);
        assert_eq!(slippage, 500); // Default 5%
    }

    #[test]
    fn test_jito_bundle() {
        let bundle = JitoBundle {
            transactions: vec![vec![1, 2, 3]],
            tip_account: Pubkey::new_unique(),
            tip_amount: 100_000,
        };

        assert_eq!(bundle.transactions.len(), 1);
        assert_eq!(bundle.tip_amount, 100_000);
    }

    #[test]
    fn test_mev_protection_with_custom_threshold() {
        let mev = MevProtection::new("http://localhost:8899")
            .with_sandwich_threshold(2000.0);

        assert!(!mev.is_sandwich_target(1500.0));
        assert!(mev.is_sandwich_target(2500.0));
    }

    #[test]
    fn test_known_dex_programs() {
        let mev = MevProtection::new("http://localhost:8899");
        let programs = mev.get_known_dex_programs();

        assert!(!programs.is_empty());

        // Should include Jupiter
        let jupiter = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
        assert!(programs.contains(&jupiter));
    }
}
