//! Token safety checker.
//!
//! Detects rug pulls and dangerous token configurations before users buy.
//! Analyzes mint authority, freeze authority, holder concentration, and more.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::{solana_program::program_pack::Pack, state::Mint};

/// Risk level for token safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenRiskLevel {
    /// Token appears safe.
    Safe,
    /// Minor concerns, proceed with caution.
    Caution,
    /// Significant red flags detected.
    Warning,
    /// Critical issues, high rug pull risk.
    Critical,
}

impl std::fmt::Display for TokenRiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenRiskLevel::Safe => write!(f, "SAFE"),
            TokenRiskLevel::Caution => write!(f, "CAUTION"),
            TokenRiskLevel::Warning => write!(f, "WARNING"),
            TokenRiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Specific risk indicator found during analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskIndicator {
    /// Mint authority is active (can print more tokens).
    ActiveMintAuthority,
    /// Freeze authority is active (can freeze wallets).
    ActiveFreezeAuthority,
    /// Top holders own significant percentage.
    HighHolderConcentration { top_10_percent: f64 },
    /// Very few unique holders.
    LowHolderCount { count: usize },
    /// Token supply is very low.
    LowSupply { supply: u64 },
    /// Token not initialized properly.
    NotInitialized,
    /// Suspicious token name/symbol pattern.
    SuspiciousMetadata { reason: String },
}

impl RiskIndicator {
    /// Get the risk weight for scoring (0-25).
    pub fn weight(&self) -> u8 {
        match self {
            RiskIndicator::ActiveMintAuthority => 20,
            RiskIndicator::ActiveFreezeAuthority => 25,
            RiskIndicator::HighHolderConcentration { top_10_percent } => {
                if *top_10_percent > 90.0 {
                    25
                } else if *top_10_percent > 70.0 {
                    15
                } else {
                    10
                }
            }
            RiskIndicator::LowHolderCount { count } => {
                if *count < 10 {
                    20
                } else if *count < 50 {
                    15
                } else {
                    10
                }
            }
            RiskIndicator::LowSupply { .. } => 10,
            RiskIndicator::NotInitialized => 25,
            RiskIndicator::SuspiciousMetadata { .. } => 15,
        }
    }

    /// Get human-readable description.
    pub fn description(&self) -> String {
        match self {
            RiskIndicator::ActiveMintAuthority => {
                "Mint authority active: token creator can print unlimited tokens".to_string()
            }
            RiskIndicator::ActiveFreezeAuthority => {
                "Freeze authority active: token creator can freeze any wallet".to_string()
            }
            RiskIndicator::HighHolderConcentration { top_10_percent } => {
                format!(
                    "High concentration: top 10 holders own {:.1}% of supply",
                    top_10_percent
                )
            }
            RiskIndicator::LowHolderCount { count } => {
                format!("Low holder count: only {} unique holders", count)
            }
            RiskIndicator::LowSupply { supply } => {
                format!("Low token supply: {}", supply)
            }
            RiskIndicator::NotInitialized => "Token mint not properly initialized".to_string(),
            RiskIndicator::SuspiciousMetadata { reason } => {
                format!("Suspicious metadata: {}", reason)
            }
        }
    }
}

/// Holder distribution analysis.
#[derive(Debug, Clone)]
pub struct HolderDistribution {
    /// Total number of unique holders.
    pub holder_count: usize,
    /// Percentage held by top holder.
    pub top_1_percent: f64,
    /// Percentage held by top 5 holders.
    pub top_5_percent: f64,
    /// Percentage held by top 10 holders.
    pub top_10_percent: f64,
    /// Top holder balances (address, amount, percentage).
    pub top_holders: Vec<(Pubkey, u64, f64)>,
}

impl Default for HolderDistribution {
    fn default() -> Self {
        Self {
            holder_count: 0,
            top_1_percent: 0.0,
            top_5_percent: 0.0,
            top_10_percent: 0.0,
            top_holders: Vec::new(),
        }
    }
}

/// Complete token safety report.
#[derive(Debug, Clone)]
pub struct TokenSafetyReport {
    /// The analyzed token mint.
    pub mint: Pubkey,
    /// Overall risk level.
    pub risk_level: TokenRiskLevel,
    /// Risk score from 0 (safe) to 100 (critical).
    pub risk_score: u8,
    /// Detected risk indicators.
    pub indicators: Vec<RiskIndicator>,
    /// Token supply.
    pub supply: u64,
    /// Token decimals.
    pub decimals: u8,
    /// Whether mint authority is active.
    pub mint_authority_active: bool,
    /// Whether freeze authority is active.
    pub freeze_authority_active: bool,
    /// Holder distribution analysis.
    pub holder_distribution: Option<HolderDistribution>,
    /// Human-readable summary.
    pub summary: String,
}

impl TokenSafetyReport {
    /// Check if token is considered safe.
    pub fn is_safe(&self) -> bool {
        self.risk_level == TokenRiskLevel::Safe
    }

    /// Check if token should be avoided.
    pub fn should_avoid(&self) -> bool {
        matches!(
            self.risk_level,
            TokenRiskLevel::Warning | TokenRiskLevel::Critical
        )
    }

    /// Get all indicator descriptions.
    pub fn get_warnings(&self) -> Vec<String> {
        self.indicators.iter().map(|i| i.description()).collect()
    }
}

/// Token safety analyzer.
pub struct TokenSafetyChecker {
    /// Minimum holder count before flagging.
    min_holder_count: usize,
    /// Maximum top 10 concentration before warning.
    max_top_10_concentration: f64,
}

impl Default for TokenSafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenSafetyChecker {
    /// Create a new token safety checker with default thresholds.
    pub fn new() -> Self {
        Self {
            min_holder_count: 100,
            max_top_10_concentration: 50.0,
        }
    }

    /// Set minimum holder count threshold.
    pub fn min_holders(mut self, count: usize) -> Self {
        self.min_holder_count = count;
        self
    }

    /// Set maximum top 10 concentration threshold (percentage).
    pub fn max_concentration(mut self, percent: f64) -> Self {
        self.max_top_10_concentration = percent;
        self
    }

    /// Analyze a token for safety issues.
    ///
    /// Performs comprehensive analysis including:
    /// - Mint authority check
    /// - Freeze authority check
    /// - Holder distribution analysis
    /// - Risk scoring
    pub fn analyze_token(&self, client: &RpcClient, mint: &Pubkey) -> Result<TokenSafetyReport> {
        // Fetch mint account data
        let account = client.get_account(mint).map_err(|e| {
            ToolkitError::AccountNotFound(format!("Mint account not found: {}", e))
        })?;

        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| ToolkitError::InvalidAccountData(format!("Invalid mint data: {}", e)))?;

        let mut indicators = Vec::new();

        // Check if initialized
        if !mint_data.is_initialized {
            indicators.push(RiskIndicator::NotInitialized);
        }

        // Check mint authority
        let mint_authority_active = mint_data.mint_authority.is_some();
        if mint_authority_active {
            indicators.push(RiskIndicator::ActiveMintAuthority);
        }

        // Check freeze authority
        let freeze_authority_active = mint_data.freeze_authority.is_some();
        if freeze_authority_active {
            indicators.push(RiskIndicator::ActiveFreezeAuthority);
        }

        // Get holder distribution
        let holder_distribution = self.get_holder_distribution(client, mint, mint_data.supply)?;

        // Check holder concentration
        if let Some(ref dist) = holder_distribution {
            if dist.top_10_percent > self.max_top_10_concentration {
                indicators.push(RiskIndicator::HighHolderConcentration {
                    top_10_percent: dist.top_10_percent,
                });
            }

            if dist.holder_count < self.min_holder_count {
                indicators.push(RiskIndicator::LowHolderCount {
                    count: dist.holder_count,
                });
            }
        }

        // Check supply
        if mint_data.supply == 0 {
            indicators.push(RiskIndicator::LowSupply {
                supply: mint_data.supply,
            });
        }

        // Calculate risk score
        let risk_score = self.calculate_risk_score(&indicators);

        // Determine risk level
        let risk_level = self.score_to_level(risk_score);

        // Generate summary
        let summary = self.generate_summary(&indicators, risk_level, risk_score);

        Ok(TokenSafetyReport {
            mint: *mint,
            risk_level,
            risk_score,
            indicators,
            supply: mint_data.supply,
            decimals: mint_data.decimals,
            mint_authority_active,
            freeze_authority_active,
            holder_distribution,
            summary,
        })
    }

    /// Check for rug pull indicators without full holder analysis.
    ///
    /// Faster than full analysis - only checks mint data.
    pub fn check_rug_pull_indicators(
        &self,
        client: &RpcClient,
        mint: &Pubkey,
    ) -> Result<Vec<RiskIndicator>> {
        let account = client.get_account(mint).map_err(|e| {
            ToolkitError::AccountNotFound(format!("Mint account not found: {}", e))
        })?;

        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| ToolkitError::InvalidAccountData(format!("Invalid mint data: {}", e)))?;

        let mut indicators = Vec::new();

        if !mint_data.is_initialized {
            indicators.push(RiskIndicator::NotInitialized);
        }

        if mint_data.mint_authority.is_some() {
            indicators.push(RiskIndicator::ActiveMintAuthority);
        }

        if mint_data.freeze_authority.is_some() {
            indicators.push(RiskIndicator::ActiveFreezeAuthority);
        }

        if mint_data.supply == 0 {
            indicators.push(RiskIndicator::LowSupply {
                supply: mint_data.supply,
            });
        }

        Ok(indicators)
    }

    /// Get holder distribution for a token.
    pub fn get_holder_distribution(
        &self,
        client: &RpcClient,
        mint: &Pubkey,
        total_supply: u64,
    ) -> Result<Option<HolderDistribution>> {
        if total_supply == 0 {
            return Ok(None);
        }

        // Get largest token accounts
        let largest_accounts = client
            .get_token_largest_accounts(mint)
            .map_err(|e| ToolkitError::NetworkError(format!("Failed to get holders: {}", e)))?;

        if largest_accounts.is_empty() {
            return Ok(Some(HolderDistribution::default()));
        }

        let mut top_holders = Vec::new();

        for account_balance in &largest_accounts {
            let amount = account_balance
                .amount
                .amount
                .parse::<u64>()
                .unwrap_or(0);
            let address = account_balance.address.parse::<Pubkey>().unwrap_or_default();
            let percentage = (amount as f64 / total_supply as f64) * 100.0;

            top_holders.push((address, amount, percentage));
        }

        let holder_count = largest_accounts.len();

        let top_1_percent = top_holders.first().map(|(_, _, p)| *p).unwrap_or(0.0);

        let top_5_percent: f64 = top_holders.iter().take(5).map(|(_, _, p)| p).sum();

        let top_10_percent: f64 = top_holders.iter().take(10).map(|(_, _, p)| p).sum();

        Ok(Some(HolderDistribution {
            holder_count,
            top_1_percent,
            top_5_percent,
            top_10_percent,
            top_holders,
        }))
    }

    /// Calculate risk score from indicators (0-100).
    pub fn calculate_risk_score(&self, indicators: &[RiskIndicator]) -> u8 {
        let total_weight: u16 = indicators.iter().map(|i| i.weight() as u16).sum();
        // Cap at 100
        total_weight.min(100) as u8
    }

    /// Convert risk score to risk level.
    fn score_to_level(&self, score: u8) -> TokenRiskLevel {
        match score {
            0..=20 => TokenRiskLevel::Safe,
            21..=40 => TokenRiskLevel::Caution,
            41..=70 => TokenRiskLevel::Warning,
            _ => TokenRiskLevel::Critical,
        }
    }

    /// Generate human-readable summary.
    fn generate_summary(
        &self,
        indicators: &[RiskIndicator],
        level: TokenRiskLevel,
        score: u8,
    ) -> String {
        if indicators.is_empty() {
            return format!("Token appears safe. Risk score: {}/100", score);
        }

        let issues: Vec<String> = indicators.iter().map(|i| i.description()).collect();

        format!(
            "Risk Level: {} (Score: {}/100)\nIssues found:\n- {}",
            level,
            score,
            issues.join("\n- ")
        )
    }

    /// Analyze token without RPC (for testing with mock data).
    pub fn analyze_from_data(
        &self,
        mint: Pubkey,
        supply: u64,
        decimals: u8,
        mint_authority: Option<Pubkey>,
        freeze_authority: Option<Pubkey>,
        holder_distribution: Option<HolderDistribution>,
    ) -> TokenSafetyReport {
        let mut indicators = Vec::new();

        // Check mint authority
        let mint_authority_active = mint_authority.is_some();
        if mint_authority_active {
            indicators.push(RiskIndicator::ActiveMintAuthority);
        }

        // Check freeze authority
        let freeze_authority_active = freeze_authority.is_some();
        if freeze_authority_active {
            indicators.push(RiskIndicator::ActiveFreezeAuthority);
        }

        // Check holder distribution
        if let Some(ref dist) = holder_distribution {
            if dist.top_10_percent > self.max_top_10_concentration {
                indicators.push(RiskIndicator::HighHolderConcentration {
                    top_10_percent: dist.top_10_percent,
                });
            }

            if dist.holder_count < self.min_holder_count {
                indicators.push(RiskIndicator::LowHolderCount {
                    count: dist.holder_count,
                });
            }
        }

        // Check supply
        if supply == 0 {
            indicators.push(RiskIndicator::LowSupply { supply });
        }

        let risk_score = self.calculate_risk_score(&indicators);
        let risk_level = self.score_to_level(risk_score);
        let summary = self.generate_summary(&indicators, risk_level, risk_score);

        TokenSafetyReport {
            mint,
            risk_level,
            risk_score,
            indicators,
            supply,
            decimals,
            mint_authority_active,
            freeze_authority_active,
            holder_distribution,
            summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_mint() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_safe_token_passes() {
        let checker = TokenSafetyChecker::new();

        // Simulate a safe token like USDC
        // - No mint authority (can't print more)
        // - No freeze authority
        // - Good holder distribution
        let distribution = HolderDistribution {
            holder_count: 1000,
            top_1_percent: 5.0,
            top_5_percent: 15.0,
            top_10_percent: 25.0,
            top_holders: vec![],
        };

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000_000_000, // 1 trillion supply
            6,                 // USDC decimals
            None,              // No mint authority
            None,              // No freeze authority
            Some(distribution),
        );

        assert!(report.is_safe());
        assert_eq!(report.risk_level, TokenRiskLevel::Safe);
        assert_eq!(report.risk_score, 0);
        assert!(report.indicators.is_empty());
    }

    #[test]
    fn test_active_mint_authority_flagged() {
        let checker = TokenSafetyChecker::new();
        let authority = Pubkey::new_unique();

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            Some(authority), // Active mint authority
            None,
            None,
        );

        assert!(report.mint_authority_active);
        assert!(report
            .indicators
            .contains(&RiskIndicator::ActiveMintAuthority));
        assert!(report.risk_score >= 20); // Mint authority weight is 20
    }

    #[test]
    fn test_active_freeze_authority_flagged() {
        let checker = TokenSafetyChecker::new();
        let authority = Pubkey::new_unique();

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            None,
            Some(authority), // Active freeze authority
            None,
        );

        assert!(report.freeze_authority_active);
        assert!(report
            .indicators
            .contains(&RiskIndicator::ActiveFreezeAuthority));
        assert!(report.risk_score >= 25); // Freeze authority weight is 25
    }

    #[test]
    fn test_high_holder_concentration_warning() {
        let checker = TokenSafetyChecker::new();

        // Top 10 holders own 75% - should trigger warning
        let distribution = HolderDistribution {
            holder_count: 500,
            top_1_percent: 30.0,
            top_5_percent: 55.0,
            top_10_percent: 75.0, // > 50% threshold
            top_holders: vec![],
        };

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            None,
            None,
            Some(distribution),
        );

        let has_concentration_warning = report.indicators.iter().any(|i| {
            matches!(i, RiskIndicator::HighHolderConcentration { .. })
        });
        assert!(has_concentration_warning);
        assert!(report.risk_score > 0);
    }

    #[test]
    fn test_low_holder_count_warning() {
        let checker = TokenSafetyChecker::new();

        // Only 50 holders - below 100 threshold
        let distribution = HolderDistribution {
            holder_count: 50,
            top_1_percent: 10.0,
            top_5_percent: 30.0,
            top_10_percent: 45.0,
            top_holders: vec![],
        };

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            None,
            None,
            Some(distribution),
        );

        let has_holder_warning = report.indicators.iter().any(|i| {
            matches!(i, RiskIndicator::LowHolderCount { count } if *count == 50)
        });
        assert!(has_holder_warning);
    }

    #[test]
    fn test_calculate_risk_score() {
        let checker = TokenSafetyChecker::new();

        // No indicators = 0 score
        let score = checker.calculate_risk_score(&[]);
        assert_eq!(score, 0);

        // Just mint authority = 20
        let score = checker.calculate_risk_score(&[RiskIndicator::ActiveMintAuthority]);
        assert_eq!(score, 20);

        // Mint + freeze = 45
        let score = checker.calculate_risk_score(&[
            RiskIndicator::ActiveMintAuthority,
            RiskIndicator::ActiveFreezeAuthority,
        ]);
        assert_eq!(score, 45);

        // Score caps at 100
        let score = checker.calculate_risk_score(&[
            RiskIndicator::ActiveMintAuthority,
            RiskIndicator::ActiveFreezeAuthority,
            RiskIndicator::HighHolderConcentration { top_10_percent: 95.0 },
            RiskIndicator::LowHolderCount { count: 5 },
            RiskIndicator::NotInitialized,
        ]);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_multiple_red_flags_critical() {
        let checker = TokenSafetyChecker::new();
        let authority = Pubkey::new_unique();

        // Token with multiple red flags
        let distribution = HolderDistribution {
            holder_count: 20, // Very low
            top_1_percent: 60.0,
            top_5_percent: 85.0,
            top_10_percent: 95.0, // Extreme concentration
            top_holders: vec![],
        };

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            Some(authority), // Can print more
            Some(authority), // Can freeze
            Some(distribution),
        );

        assert_eq!(report.risk_level, TokenRiskLevel::Critical);
        assert!(report.risk_score >= 70);
        assert!(report.should_avoid());
        assert!(report.indicators.len() >= 3);
    }

    #[test]
    fn test_risk_level_thresholds() {
        let checker = TokenSafetyChecker::new();

        // 0-20 = Safe
        assert_eq!(checker.score_to_level(0), TokenRiskLevel::Safe);
        assert_eq!(checker.score_to_level(20), TokenRiskLevel::Safe);

        // 21-40 = Caution
        assert_eq!(checker.score_to_level(21), TokenRiskLevel::Caution);
        assert_eq!(checker.score_to_level(40), TokenRiskLevel::Caution);

        // 41-70 = Warning
        assert_eq!(checker.score_to_level(41), TokenRiskLevel::Warning);
        assert_eq!(checker.score_to_level(70), TokenRiskLevel::Warning);

        // 71+ = Critical
        assert_eq!(checker.score_to_level(71), TokenRiskLevel::Critical);
        assert_eq!(checker.score_to_level(100), TokenRiskLevel::Critical);
    }

    #[test]
    fn test_indicator_descriptions() {
        let mint_auth = RiskIndicator::ActiveMintAuthority;
        assert!(mint_auth.description().contains("print"));

        let freeze = RiskIndicator::ActiveFreezeAuthority;
        assert!(freeze.description().contains("freeze"));

        let concentration = RiskIndicator::HighHolderConcentration { top_10_percent: 80.0 };
        assert!(concentration.description().contains("80.0%"));

        let holders = RiskIndicator::LowHolderCount { count: 50 };
        assert!(holders.description().contains("50"));
    }

    #[test]
    fn test_report_summary() {
        let checker = TokenSafetyChecker::new();
        let authority = Pubkey::new_unique();

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            Some(authority),
            None,
            None,
        );

        assert!(report.summary.contains("Risk Level"));
        assert!(report.summary.contains("Mint authority"));
    }

    #[test]
    fn test_custom_thresholds() {
        // Use stricter thresholds
        let checker = TokenSafetyChecker::new()
            .min_holders(500)        // Require 500 holders
            .max_concentration(30.0); // Flag if top 10 > 30%

        let distribution = HolderDistribution {
            holder_count: 200, // Below 500
            top_1_percent: 10.0,
            top_5_percent: 25.0,
            top_10_percent: 35.0, // Above 30%
            top_holders: vec![],
        };

        let report = checker.analyze_from_data(
            mock_mint(),
            1_000_000,
            9,
            None,
            None,
            Some(distribution),
        );

        // Should flag both low holders and concentration
        assert!(report.indicators.len() >= 2);
    }
}
