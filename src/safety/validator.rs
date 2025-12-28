//! Safety protocol orchestrator.
//!
//! Combines address verification and amount validation into a unified
//! safety check for transfers.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::{solana_program::program_pack::Pack, state::Account as TokenAccount};

use super::address_verify::AddressVerifier;
use super::amount_validation::AmountValidator;
use super::token_safety::{
    HolderDistribution, RiskIndicator, TokenRiskLevel, TokenSafetyChecker, TokenSafetyReport,
};

/// Risk level for a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// No concerns detected.
    Low,
    /// Minor warnings, proceed with caution.
    Medium,
    /// Significant concerns, requires confirmation.
    High,
    /// Critical issues, transaction blocked.
    Critical,
}

impl RiskLevel {
    /// Returns true if this risk level should block the transaction.
    pub fn is_blocking(&self) -> bool {
        matches!(self, RiskLevel::Critical)
    }

    /// Returns true if this risk level requires user confirmation.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Complete safety report for a transaction.
#[derive(Debug, Clone)]
pub struct SafetyReport {
    /// Whether the transaction is approved to proceed.
    pub approved: bool,
    /// Overall risk level.
    pub risk_level: RiskLevel,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
    /// Blocking issues that prevent the transaction.
    pub blockers: Vec<String>,
    /// Sender address in short format.
    pub from_display: String,
    /// Recipient address in short format.
    pub to_display: String,
    /// Human-readable amount.
    pub amount_display: String,
    /// Whether user confirmation is required.
    pub requires_confirmation: bool,
    /// Token safety report (for token transfers).
    pub token_report: Option<TokenSafetyReport>,
}

impl SafetyReport {
    /// Create a new approved report with low risk.
    fn approved(from: &Pubkey, to: &Pubkey, amount_display: String) -> Self {
        Self {
            approved: true,
            risk_level: RiskLevel::Low,
            warnings: Vec::new(),
            blockers: Vec::new(),
            from_display: AddressVerifier::format_address_short(from),
            to_display: AddressVerifier::format_address_short(to),
            amount_display,
            requires_confirmation: false,
            token_report: None,
        }
    }

    /// Set the token report.
    fn with_token_report(mut self, report: TokenSafetyReport) -> Self {
        self.token_report = Some(report);
        self
    }

    /// Add a warning and adjust risk level.
    fn add_warning(&mut self, warning: String, level: RiskLevel) {
        self.warnings.push(warning);
        if level > self.risk_level {
            self.risk_level = level;
        }
        if self.risk_level.requires_confirmation() {
            self.requires_confirmation = true;
        }
    }

    /// Add a blocker and mark as not approved.
    fn add_blocker(&mut self, blocker: String) {
        self.blockers.push(blocker);
        self.approved = false;
        self.risk_level = RiskLevel::Critical;
    }

    /// Format report for display.
    pub fn summary(&self) -> String {
        let status = if self.approved { "APPROVED" } else { "BLOCKED" };
        let mut lines = vec![
            format!("Safety Report: {} (Risk: {})", status, self.risk_level),
            format!("Transfer: {} -> {}", self.from_display, self.to_display),
            format!("Amount: {}", self.amount_display),
        ];

        if !self.warnings.is_empty() {
            lines.push(format!("Warnings ({}):", self.warnings.len()));
            for w in &self.warnings {
                lines.push(format!("  - {}", w));
            }
        }

        if !self.blockers.is_empty() {
            lines.push(format!("Blockers ({}):", self.blockers.len()));
            for b in &self.blockers {
                lines.push(format!("  - {}", b));
            }
        }

        lines.join("\n")
    }
}

/// Safety protocol for validating transactions.
pub struct SafetyProtocol {
    /// Whether to use strict mode (block on any warning).
    strict_mode: bool,
    /// USD threshold for requiring confirmation.
    large_amount_threshold_usd: f64,
    /// Estimated token price in USD (for large amount checks).
    token_price_usd: Option<f64>,
}

impl Default for SafetyProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetyProtocol {
    /// Create a new SafetyProtocol with default settings.
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            large_amount_threshold_usd: 1000.0,
            token_price_usd: None,
        }
    }

    /// Enable strict mode (block on any warning).
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Set the large amount threshold in USD.
    pub fn large_amount_threshold(mut self, threshold: f64) -> Self {
        self.large_amount_threshold_usd = threshold;
        self
    }

    /// Set the token price for USD calculations.
    pub fn token_price(mut self, price: f64) -> Self {
        self.token_price_usd = Some(price);
        self
    }

    /// Validate a transfer for safety issues.
    ///
    /// Performs the following checks:
    /// 1. Verify sender and recipient addresses
    /// 2. Check sender has sufficient balance
    /// 3. Validate amount (not zero, not exceeding balance)
    /// 4. Check for full balance sends
    /// 5. Check for large amounts requiring confirmation
    ///
    /// # Arguments
    /// * `client` - RPC client for balance queries
    /// * `from` - Sender pubkey
    /// * `to` - Recipient pubkey
    /// * `amount` - Amount in base units (lamports for SOL)
    /// * `decimals` - Token decimals (9 for SOL)
    ///
    /// # Returns
    /// SafetyReport with approval status and any warnings/blockers
    pub async fn validate_transfer(
        &self,
        client: &RpcClient,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<SafetyReport> {
        let amount_display = AmountValidator::format_amount(amount, decimals);
        let mut report = SafetyReport::approved(from, to, amount_display.clone());

        // 1. Verify addresses are valid
        if let Err(e) = AddressVerifier::verify_address(&from.to_string()) {
            report.add_blocker(format!("Invalid sender address: {}", e));
        }

        if let Err(e) = AddressVerifier::verify_address(&to.to_string()) {
            report.add_blocker(format!("Invalid recipient address: {}", e));
        }

        // Check for self-transfer
        if from == to {
            report.add_warning(
                "Sending to yourself".to_string(),
                RiskLevel::Medium,
            );
        }

        // 2. Fetch balance and validate amount
        let balance = client.get_balance(from).map_err(|e| {
            ToolkitError::NetworkError(format!("Failed to fetch balance: {}", e))
        })?;

        // 3. Validate amount against balance
        let validation = AmountValidator::validate_amount(amount, decimals, balance);

        if !validation.is_valid {
            for warning in &validation.warnings {
                if warning.contains("exceeds balance") || warning.contains("zero") {
                    report.add_blocker(warning.clone());
                }
            }
        }

        // 4. Add non-blocking warnings
        for warning in &validation.warnings {
            if !warning.contains("exceeds balance") && !warning.contains("zero") {
                let level = if warning.contains("entire balance") {
                    RiskLevel::High
                } else if warning.contains("%") {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };
                report.add_warning(warning.clone(), level);
            }
        }

        // 5. Check for large amounts requiring confirmation
        if let Some(price) = self.token_price_usd {
            let human_amount = AmountValidator::token_to_human_amount(amount, decimals);
            let usd_value = human_amount * price;

            if AmountValidator::requires_confirmation(usd_value, self.large_amount_threshold_usd) {
                report.add_warning(
                    format!(
                        "Large transfer: ~${:.2} USD exceeds ${:.0} threshold",
                        usd_value, self.large_amount_threshold_usd
                    ),
                    RiskLevel::High,
                );
            }
        }

        // In strict mode, any warning becomes a blocker
        if self.strict_mode && !report.warnings.is_empty() {
            let warnings: Vec<String> = report.warnings.drain(..).collect();
            for warning in warnings {
                report.add_blocker(format!("STRICT: {}", warning));
            }
        }

        Ok(report)
    }

    /// Validate a transfer synchronously (blocking).
    pub fn validate_transfer_sync(
        &self,
        client: &RpcClient,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<SafetyReport> {
        // Use tokio runtime if available, otherwise create one
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ToolkitError::Custom(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(self.validate_transfer(client, from, to, amount, decimals))
    }

    /// Quick validation without RPC calls (for testing or offline checks).
    ///
    /// Only validates addresses and amount format, does not check balance.
    pub fn validate_offline(
        &self,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
        decimals: u8,
        balance: u64,
    ) -> SafetyReport {
        let amount_display = AmountValidator::format_amount(amount, decimals);
        let mut report = SafetyReport::approved(from, to, amount_display);

        // Verify addresses
        if let Err(e) = AddressVerifier::verify_address(&from.to_string()) {
            report.add_blocker(format!("Invalid sender address: {}", e));
        }

        if let Err(e) = AddressVerifier::verify_address(&to.to_string()) {
            report.add_blocker(format!("Invalid recipient address: {}", e));
        }

        // Check for self-transfer
        if from == to {
            report.add_warning("Sending to yourself".to_string(), RiskLevel::Medium);
        }

        // Validate amount
        let validation = AmountValidator::validate_amount(amount, decimals, balance);

        if !validation.is_valid {
            for warning in &validation.warnings {
                if warning.contains("exceeds balance") || warning.contains("zero") {
                    report.add_blocker(warning.clone());
                }
            }
        }

        for warning in &validation.warnings {
            if !warning.contains("exceeds balance") && !warning.contains("zero") {
                let level = if warning.contains("entire balance") {
                    RiskLevel::High
                } else if warning.contains("%") {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };
                report.add_warning(warning.clone(), level);
            }
        }

        // Check for large amounts
        if let Some(price) = self.token_price_usd {
            let human_amount = AmountValidator::token_to_human_amount(amount, decimals);
            let usd_value = human_amount * price;

            if AmountValidator::requires_confirmation(usd_value, self.large_amount_threshold_usd) {
                report.add_warning(
                    format!(
                        "Large transfer: ~${:.2} USD exceeds ${:.0} threshold",
                        usd_value, self.large_amount_threshold_usd
                    ),
                    RiskLevel::High,
                );
            }
        }

        // Strict mode
        if self.strict_mode && !report.warnings.is_empty() {
            let warnings: Vec<String> = report.warnings.drain(..).collect();
            for warning in warnings {
                report.add_blocker(format!("STRICT: {}", warning));
            }
        }

        report
    }

    /// Validate a token transfer including token safety analysis.
    ///
    /// Performs comprehensive validation:
    /// 1. Analyze token for rug pull indicators
    /// 2. Verify sender and recipient addresses
    /// 3. Check token account balance
    /// 4. Validate amount
    /// 5. Aggregate all risks into SafetyReport
    ///
    /// # Arguments
    /// * `client` - RPC client for queries
    /// * `token_mint` - Token mint address
    /// * `from` - Sender wallet pubkey
    /// * `to` - Recipient wallet pubkey
    /// * `amount` - Amount in base units
    ///
    /// # Returns
    /// SafetyReport with token risks, address validation, and amount checks
    pub fn validate_token_transfer(
        &self,
        client: &RpcClient,
        token_mint: &Pubkey,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
    ) -> Result<SafetyReport> {
        // 1. Analyze token safety
        let token_checker = TokenSafetyChecker::new();
        let token_report = token_checker.analyze_token(client, token_mint)?;

        let decimals = token_report.decimals;
        let amount_display = AmountValidator::format_amount(amount, decimals);

        let mut report = SafetyReport::approved(from, to, amount_display)
            .with_token_report(token_report.clone());

        // Add token risk warnings based on indicators
        for indicator in &token_report.indicators {
            let (warning, level) = self.indicator_to_warning(indicator);
            report.add_warning(warning, level);
        }

        // If token is critical risk, block the transfer
        if token_report.risk_level == TokenRiskLevel::Critical {
            report.add_blocker(format!(
                "Token has critical risk score: {}/100",
                token_report.risk_score
            ));
        }

        // 2. Verify addresses
        if let Err(e) = AddressVerifier::verify_address(&from.to_string()) {
            report.add_blocker(format!("Invalid sender address: {}", e));
        }

        if let Err(e) = AddressVerifier::verify_address(&to.to_string()) {
            report.add_blocker(format!("Invalid recipient address: {}", e));
        }

        if from == to {
            report.add_warning("Sending to yourself".to_string(), RiskLevel::Medium);
        }

        // 3. Get token account balance
        let from_ata = spl_associated_token_account::get_associated_token_address(from, token_mint);
        let balance = match client.get_account(&from_ata) {
            Ok(account) => {
                TokenAccount::unpack(&account.data)
                    .map(|ta| ta.amount)
                    .unwrap_or(0)
            }
            Err(_) => 0,
        };

        // 4. Validate amount
        let validation = AmountValidator::validate_amount(amount, decimals, balance);

        if !validation.is_valid {
            for warning in &validation.warnings {
                if warning.contains("exceeds balance") || warning.contains("zero") {
                    report.add_blocker(warning.clone());
                }
            }
        }

        for warning in &validation.warnings {
            if !warning.contains("exceeds balance") && !warning.contains("zero") {
                let level = if warning.contains("entire balance") {
                    RiskLevel::High
                } else if warning.contains("%") {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };
                report.add_warning(warning.clone(), level);
            }
        }

        // 5. Check large amounts
        if let Some(price) = self.token_price_usd {
            let human_amount = AmountValidator::token_to_human_amount(amount, decimals);
            let usd_value = human_amount * price;

            if AmountValidator::requires_confirmation(usd_value, self.large_amount_threshold_usd) {
                report.add_warning(
                    format!(
                        "Large transfer: ~${:.2} USD exceeds ${:.0} threshold",
                        usd_value, self.large_amount_threshold_usd
                    ),
                    RiskLevel::High,
                );
            }
        }

        // Strict mode
        if self.strict_mode && !report.warnings.is_empty() {
            let warnings: Vec<String> = report.warnings.drain(..).collect();
            for warning in warnings {
                report.add_blocker(format!("STRICT: {}", warning));
            }
        }

        Ok(report)
    }

    /// Validate token transfer without RPC (for testing).
    ///
    /// Uses provided token data instead of fetching from chain.
    pub fn validate_token_transfer_offline(
        &self,
        token_mint: Pubkey,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
        balance: u64,
        decimals: u8,
        mint_authority: Option<Pubkey>,
        freeze_authority: Option<Pubkey>,
        holder_distribution: Option<HolderDistribution>,
    ) -> SafetyReport {
        // Create token report from provided data
        let token_checker = TokenSafetyChecker::new();
        let token_report = token_checker.analyze_from_data(
            token_mint,
            1_000_000_000, // Dummy supply
            decimals,
            mint_authority,
            freeze_authority,
            holder_distribution,
        );

        let amount_display = AmountValidator::format_amount(amount, decimals);
        let mut report = SafetyReport::approved(from, to, amount_display)
            .with_token_report(token_report.clone());

        // Add token risk warnings
        for indicator in &token_report.indicators {
            let (warning, level) = self.indicator_to_warning(indicator);
            report.add_warning(warning, level);
        }

        if token_report.risk_level == TokenRiskLevel::Critical {
            report.add_blocker(format!(
                "Token has critical risk score: {}/100",
                token_report.risk_score
            ));
        }

        // Verify addresses
        if let Err(e) = AddressVerifier::verify_address(&from.to_string()) {
            report.add_blocker(format!("Invalid sender address: {}", e));
        }

        if let Err(e) = AddressVerifier::verify_address(&to.to_string()) {
            report.add_blocker(format!("Invalid recipient address: {}", e));
        }

        if from == to {
            report.add_warning("Sending to yourself".to_string(), RiskLevel::Medium);
        }

        // Validate amount
        let validation = AmountValidator::validate_amount(amount, decimals, balance);

        if !validation.is_valid {
            for warning in &validation.warnings {
                if warning.contains("exceeds balance") || warning.contains("zero") {
                    report.add_blocker(warning.clone());
                }
            }
        }

        for warning in &validation.warnings {
            if !warning.contains("exceeds balance") && !warning.contains("zero") {
                let level = if warning.contains("entire balance") {
                    RiskLevel::High
                } else if warning.contains("%") {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };
                report.add_warning(warning.clone(), level);
            }
        }

        // Large amount check
        if let Some(price) = self.token_price_usd {
            let human_amount = AmountValidator::token_to_human_amount(amount, decimals);
            let usd_value = human_amount * price;

            if AmountValidator::requires_confirmation(usd_value, self.large_amount_threshold_usd) {
                report.add_warning(
                    format!(
                        "Large transfer: ~${:.2} USD exceeds ${:.0} threshold",
                        usd_value, self.large_amount_threshold_usd
                    ),
                    RiskLevel::High,
                );
            }
        }

        // Strict mode
        if self.strict_mode && !report.warnings.is_empty() {
            let warnings: Vec<String> = report.warnings.drain(..).collect();
            for warning in warnings {
                report.add_blocker(format!("STRICT: {}", warning));
            }
        }

        report
    }

    /// Convert token risk indicator to warning message and level.
    fn indicator_to_warning(&self, indicator: &RiskIndicator) -> (String, RiskLevel) {
        match indicator {
            RiskIndicator::ActiveMintAuthority => (
                "Token has active mint authority (can print unlimited tokens)".to_string(),
                RiskLevel::High,
            ),
            RiskIndicator::ActiveFreezeAuthority => (
                "Token has active freeze authority (can freeze your wallet)".to_string(),
                RiskLevel::High,
            ),
            RiskIndicator::HighHolderConcentration { top_10_percent } => (
                format!(
                    "High holder concentration: top 10 own {:.1}% of supply",
                    top_10_percent
                ),
                if *top_10_percent > 80.0 {
                    RiskLevel::High
                } else {
                    RiskLevel::Medium
                },
            ),
            RiskIndicator::LowHolderCount { count } => (
                format!("Low holder count: only {} holders", count),
                if *count < 50 {
                    RiskLevel::High
                } else {
                    RiskLevel::Medium
                },
            ),
            RiskIndicator::LowSupply { supply } => (
                format!("Token has zero or very low supply: {}", supply),
                RiskLevel::Medium,
            ),
            RiskIndicator::NotInitialized => (
                "Token mint not properly initialized".to_string(),
                RiskLevel::Critical,
            ),
            RiskIndicator::SuspiciousMetadata { reason } => (
                format!("Suspicious token metadata: {}", reason),
                RiskLevel::Medium,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::amount_validation::LAMPORTS_PER_SOL;
    use std::str::FromStr;

    const TEST_ADDR_1: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
    const TEST_ADDR_2: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    fn test_pubkey_1() -> Pubkey {
        Pubkey::from_str(TEST_ADDR_1).unwrap()
    }

    fn test_pubkey_2() -> Pubkey {
        Pubkey::from_str(TEST_ADDR_2).unwrap()
    }

    #[test]
    fn test_safety_protocol_new() {
        let protocol = SafetyProtocol::new();
        assert!(!protocol.strict_mode);
        assert_eq!(protocol.large_amount_threshold_usd, 1000.0);
    }

    #[test]
    fn test_safety_protocol_builder() {
        let protocol = SafetyProtocol::new()
            .strict()
            .large_amount_threshold(500.0)
            .token_price(100.0);

        assert!(protocol.strict_mode);
        assert_eq!(protocol.large_amount_threshold_usd, 500.0);
        assert_eq!(protocol.token_price_usd, Some(100.0));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_blocking() {
        assert!(!RiskLevel::Low.is_blocking());
        assert!(!RiskLevel::Medium.is_blocking());
        assert!(!RiskLevel::High.is_blocking());
        assert!(RiskLevel::Critical.is_blocking());
    }

    #[test]
    fn test_risk_level_confirmation() {
        assert!(!RiskLevel::Low.requires_confirmation());
        assert!(!RiskLevel::Medium.requires_confirmation());
        assert!(RiskLevel::High.requires_confirmation());
        assert!(RiskLevel::Critical.requires_confirmation());
    }

    #[test]
    fn test_valid_transfer_approved() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 1 * LAMPORTS_PER_SOL;

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(report.approved);
        assert_eq!(report.risk_level, RiskLevel::Low);
        assert!(report.warnings.is_empty());
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn test_insufficient_balance_blocked() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 1 * LAMPORTS_PER_SOL;
        let amount = 5 * LAMPORTS_PER_SOL;

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(!report.approved);
        assert_eq!(report.risk_level, RiskLevel::Critical);
        assert!(report.blockers.iter().any(|b| b.contains("exceeds balance")));
    }

    #[test]
    fn test_zero_amount_blocked() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 0;

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(!report.approved);
        assert!(report.blockers.iter().any(|b| b.contains("zero")));
    }

    #[test]
    fn test_full_balance_warning() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = balance; // 100%

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(report.approved);
        assert!(report.risk_level >= RiskLevel::High);
        assert!(report.warnings.iter().any(|w| w.contains("entire balance")));
        assert!(report.requires_confirmation);
    }

    #[test]
    fn test_large_amount_warning() {
        let protocol = SafetyProtocol::new()
            .token_price(100.0) // $100 per token
            .large_amount_threshold(1000.0);

        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 100 * LAMPORTS_PER_SOL;
        let amount = 15 * LAMPORTS_PER_SOL; // $1500

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(report.approved);
        assert!(report.risk_level >= RiskLevel::High);
        assert!(report.warnings.iter().any(|w| w.contains("Large transfer")));
        assert!(report.requires_confirmation);
    }

    #[test]
    fn test_self_transfer_warning() {
        let protocol = SafetyProtocol::new();
        let addr = test_pubkey_1();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 1 * LAMPORTS_PER_SOL;

        let report = protocol.validate_offline(&addr, &addr, amount, 9, balance);

        assert!(report.approved);
        assert!(report.risk_level >= RiskLevel::Medium);
        assert!(report.warnings.iter().any(|w| w.contains("yourself")));
    }

    #[test]
    fn test_strict_mode_blocks_warnings() {
        let protocol = SafetyProtocol::new().strict();
        let addr = test_pubkey_1();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 1 * LAMPORTS_PER_SOL;

        // Self-transfer triggers warning, which becomes blocker in strict mode
        let report = protocol.validate_offline(&addr, &addr, amount, 9, balance);

        assert!(!report.approved);
        assert_eq!(report.risk_level, RiskLevel::Critical);
        assert!(report.blockers.iter().any(|b| b.contains("STRICT")));
    }

    #[test]
    fn test_report_summary() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 1 * LAMPORTS_PER_SOL;

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);
        let summary = report.summary();

        assert!(summary.contains("APPROVED"));
        assert!(summary.contains("LOW"));
        assert!(summary.contains(&report.from_display));
        assert!(summary.contains(&report.to_display));
    }

    #[test]
    fn test_amount_display_format() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = 1_500_000_000; // 1.5 SOL

        let report = protocol.validate_offline(&from, &to, amount, 9, balance);

        assert!(report.amount_display.contains("1.5"));
    }

    #[test]
    fn test_multiple_warnings_highest_risk() {
        let protocol = SafetyProtocol::new()
            .token_price(100.0)
            .large_amount_threshold(500.0);

        let addr = test_pubkey_1();

        let balance = 10 * LAMPORTS_PER_SOL;
        let amount = balance; // Self-transfer of entire balance (high value)

        let report = protocol.validate_offline(&addr, &addr, amount, 9, balance);

        // Should have multiple warnings
        assert!(report.warnings.len() >= 2);
        // Risk level should be highest of all warnings
        assert!(report.risk_level >= RiskLevel::High);
    }

    // Token transfer validation tests

    #[test]
    fn test_safe_token_transfer_approved() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();

        // Safe token: no authorities, good distribution
        let distribution = HolderDistribution {
            holder_count: 500,
            top_1_percent: 5.0,
            top_5_percent: 15.0,
            top_10_percent: 25.0,
            top_holders: vec![],
        };

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000, // 1 token (6 decimals)
            10_000_000, // 10 tokens balance
            6,
            None, // No mint authority
            None, // No freeze authority
            Some(distribution),
        );

        assert!(report.approved);
        assert!(report.token_report.is_some());
        assert_eq!(report.token_report.as_ref().unwrap().risk_level, TokenRiskLevel::Safe);
    }

    #[test]
    fn test_token_with_mint_authority_warning() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            Some(authority), // Active mint authority
            None,
            None,
        );

        assert!(report.approved); // Still approved but with warning
        assert!(report.warnings.iter().any(|w| w.contains("mint authority")));
        assert!(report.risk_level >= RiskLevel::High);
    }

    #[test]
    fn test_token_with_freeze_authority_warning() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            None,
            Some(authority), // Active freeze authority
            None,
        );

        assert!(report.approved);
        assert!(report.warnings.iter().any(|w| w.contains("freeze")));
    }

    #[test]
    fn test_token_high_concentration_warning() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();

        // Dangerous distribution: top 10 own 85%
        let distribution = HolderDistribution {
            holder_count: 200,
            top_1_percent: 40.0,
            top_5_percent: 70.0,
            top_10_percent: 85.0,
            top_holders: vec![],
        };

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            None,
            None,
            Some(distribution),
        );

        assert!(report.warnings.iter().any(|w| w.contains("concentration")));
    }

    #[test]
    fn test_token_low_holder_count_warning() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();

        // Only 30 holders
        let distribution = HolderDistribution {
            holder_count: 30,
            top_1_percent: 10.0,
            top_5_percent: 30.0,
            top_10_percent: 45.0,
            top_holders: vec![],
        };

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            None,
            None,
            Some(distribution),
        );

        assert!(report.warnings.iter().any(|w| w.contains("holder")));
    }

    #[test]
    fn test_critical_token_blocked() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        // Token with multiple red flags = critical
        let distribution = HolderDistribution {
            holder_count: 10, // Very low
            top_1_percent: 60.0,
            top_5_percent: 85.0,
            top_10_percent: 95.0, // Extreme concentration
            top_holders: vec![],
        };

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            Some(authority), // Can print more
            Some(authority), // Can freeze
            Some(distribution),
        );

        // Critical risk tokens should be blocked
        assert!(!report.approved);
        assert!(report.blockers.iter().any(|b| b.contains("critical risk")));
    }

    #[test]
    fn test_token_transfer_with_amount_validation() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();

        // Try to send more than balance
        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            100_000_000, // 100 tokens
            10_000_000,  // Only 10 tokens balance
            6,
            None,
            None,
            None,
        );

        assert!(!report.approved);
        assert!(report.blockers.iter().any(|b| b.contains("exceeds balance")));
    }

    #[test]
    fn test_token_transfer_includes_token_report() {
        let protocol = SafetyProtocol::new();
        let from = test_pubkey_1();
        let to = test_pubkey_2();
        let mint = Pubkey::new_unique();

        let report = protocol.validate_token_transfer_offline(
            mint,
            &from,
            &to,
            1_000_000,
            10_000_000,
            6,
            None,
            None,
            None,
        );

        // Token report should be included
        assert!(report.token_report.is_some());
        let token_report = report.token_report.unwrap();
        assert_eq!(token_report.mint, mint);
        assert_eq!(token_report.decimals, 6);
    }
}
