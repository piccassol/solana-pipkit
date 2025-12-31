//! Analytics and monitoring utilities for Solana wallets and tokens.
//!
//! Track whale movements, analyze holder distributions, recognize trading patterns,
//! and calculate PnL for positions.

mod wallet;
mod holders;
mod patterns;
mod pnl;

pub use wallet::*;
pub use holders::*;
pub use patterns::*;
pub use pnl::*;

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::Result;

/// Main analytics interface combining all analysis capabilities.
pub struct Analytics {
    client: RpcClient,
    config: AnalyticsConfig,
}

#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// Minimum SOL balance to be considered a whale
    pub whale_threshold_sol: f64,
    /// Minimum token percentage to be considered a major holder
    pub major_holder_threshold_pct: f64,
    /// Number of transactions to analyze for patterns
    pub pattern_lookback: usize,
    /// Whether to track known smart money wallets
    pub track_smart_money: bool,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            whale_threshold_sol: 1000.0,
            major_holder_threshold_pct: 1.0,
            pattern_lookback: 100,
            track_smart_money: true,
        }
    }
}

impl Analytics {
    pub fn new(client: RpcClient) -> Self {
        Self {
            client,
            config: AnalyticsConfig::default(),
        }
    }

    pub fn with_config(client: RpcClient, config: AnalyticsConfig) -> Self {
        Self { client, config }
    }

    /// Get a full profile of a wallet including balance, activity, and classification.
    pub async fn profile_wallet(&self, wallet: &Pubkey) -> Result<WalletProfile> {
        let profiler = WalletProfiler::new(&self.client, self.config.clone());
        profiler.analyze(wallet).await
    }

    /// Analyze token holder distribution.
    pub async fn analyze_holders(&self, mint: &Pubkey) -> Result<HolderAnalysis> {
        let analyzer = HolderAnalyzer::new(&self.client);
        analyzer.analyze(mint).await
    }

    /// Detect trading patterns for a wallet.
    pub async fn detect_patterns(&self, wallet: &Pubkey) -> Result<Vec<TradingPattern>> {
        let detector = PatternDetector::new(&self.client, self.config.pattern_lookback);
        detector.analyze(wallet).await
    }

    /// Calculate PnL for a wallet's token positions.
    pub async fn calculate_pnl(&self, wallet: &Pubkey) -> Result<PnlReport> {
        let tracker = PnlTracker::new(&self.client);
        tracker.calculate(wallet).await
    }
}
