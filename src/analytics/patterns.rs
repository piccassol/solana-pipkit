//! Transaction pattern recognition for wallet behavior analysis.

use solana_client::rpc_client::{RpcClient, GetConfirmedSignaturesForAddress2Config};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use crate::{Result, ToolkitError};

#[derive(Debug, Clone)]
pub struct TradingPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub occurrences: usize,
    pub description: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    // Trading patterns
    Sniper,          // Buys immediately after liquidity add
    Scalper,         // Many small quick trades
    SwingTrader,     // Holds for days/weeks
    Holder,          // Long-term holder
    
    // Suspicious patterns
    WashTrading,     // Trading with self/related wallets
    PumpAndDump,     // Buy, promote, sell
    FrontRunner,     // MEV-style front running
    Sandwich,        // Sandwich attack pattern
    
    // Bot patterns
    ArbitrageBot,    // Cross-DEX arbitrage
    LiquidationBot,  // Liquidation hunting
    CopyTrader,      // Copies other wallets
    
    // Organic patterns
    DcaBuyer,        // Regular purchase intervals
    Accumulator,     // Gradual position building
    Distributor,     // Gradual selling
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TransactionSummary {
    signature: Signature,
    timestamp: i64,
    is_swap: bool,
    is_transfer: bool,
    token_mint: Option<Pubkey>,
    amount: Option<u64>,
    counterparty: Option<Pubkey>,
}

pub struct PatternDetector<'a> {
    client: &'a RpcClient,
    lookback: usize,
}

impl<'a> PatternDetector<'a> {
    pub fn new(client: &'a RpcClient, lookback: usize) -> Self {
        Self { client, lookback }
    }

    pub async fn analyze(&self, wallet: &Pubkey) -> Result<Vec<TradingPattern>> {
        let signatures = self.client
            .get_signatures_for_address_with_config(
                wallet,
                GetConfirmedSignaturesForAddress2Config {
                    limit: Some(self.lookback),
                    ..Default::default()
                },
            )
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        if signatures.is_empty() {
            return Ok(Vec::new());
        }

        let mut patterns = Vec::new();

        // Analyze timing patterns
        if let Some(pattern) = self.detect_timing_pattern(&signatures) {
            patterns.push(pattern);
        }

        // Analyze frequency patterns
        if let Some(pattern) = self.detect_frequency_pattern(&signatures) {
            patterns.push(pattern);
        }

        // Analyze transaction clustering
        if let Some(pattern) = self.detect_clustering(&signatures) {
            patterns.push(pattern);
        }

        // Check for sniper behavior
        if let Some(pattern) = self.detect_sniper_pattern(&signatures) {
            patterns.push(pattern);
        }

        // Check for DCA pattern
        if let Some(pattern) = self.detect_dca_pattern(&signatures) {
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    fn detect_timing_pattern(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> Option<TradingPattern> {
        if signatures.len() < 10 {
            return None;
        }

        let timestamps: Vec<i64> = signatures
            .iter()
            .filter_map(|s| s.block_time)
            .collect();

        if timestamps.len() < 10 {
            return None;
        }

        // Calculate intervals between transactions
        let intervals: Vec<i64> = timestamps
            .windows(2)
            .map(|w| (w[0] - w[1]).abs())
            .collect();

        let avg_interval = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;

        // Very fast trading (< 1 minute average)
        if avg_interval < 60.0 {
            return Some(TradingPattern {
                pattern_type: PatternType::Scalper,
                confidence: 0.8,
                occurrences: signatures.len(),
                description: format!(
                    "High-frequency trading detected. Average {} seconds between transactions.",
                    avg_interval as i64
                ),
                risk_level: RiskLevel::Medium,
            });
        }

        None
    }

    fn detect_frequency_pattern(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> Option<TradingPattern> {
        if signatures.len() < 5 {
            return None;
        }

        let _now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Count transactions per day
        let mut daily_counts: HashMap<i64, usize> = HashMap::new();
        for sig in signatures {
            if let Some(ts) = sig.block_time {
                let day = ts / 86400;
                *daily_counts.entry(day).or_insert(0) += 1;
            }
        }

        if daily_counts.is_empty() {
            return None;
        }

        let avg_per_day = signatures.len() as f64 / daily_counts.len() as f64;

        if avg_per_day > 50.0 {
            return Some(TradingPattern {
                pattern_type: PatternType::ArbitrageBot,
                confidence: 0.7,
                occurrences: signatures.len(),
                description: format!(
                    "Bot-like activity detected. Average {:.1} transactions per active day.",
                    avg_per_day
                ),
                risk_level: RiskLevel::Low,
            });
        }

        None
    }

    fn detect_clustering(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> Option<TradingPattern> {
        if signatures.len() < 20 {
            return None;
        }

        let timestamps: Vec<i64> = signatures
            .iter()
            .filter_map(|s| s.block_time)
            .collect();

        // Look for bursts of activity (many txs within short windows)
        let mut burst_count = 0;
        for window in timestamps.windows(5) {
            if window.len() == 5 {
                let spread = (window[0] - window[4]).abs();
                if spread < 300 { // 5 txs within 5 minutes
                    burst_count += 1;
                }
            }
        }

        if burst_count > 3 {
            return Some(TradingPattern {
                pattern_type: PatternType::Sniper,
                confidence: 0.6,
                occurrences: burst_count,
                description: format!(
                    "Burst trading pattern detected. {} instances of rapid-fire transactions.",
                    burst_count
                ),
                risk_level: RiskLevel::Medium,
            });
        }

        None
    }

    fn detect_sniper_pattern(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> Option<TradingPattern> {
        // Snipers typically have:
        // 1. Very fast execution after token launch
        // 2. Immediate sells after price increase
        // 3. Consistent pattern across multiple tokens

        if signatures.len() < 10 {
            return None;
        }

        // Check for multiple quick buy-sell cycles
        let timestamps: Vec<i64> = signatures
            .iter()
            .filter_map(|s| s.block_time)
            .collect();

        let mut quick_cycles = 0;
        for window in timestamps.windows(2) {
            if window.len() == 2 {
                let interval = (window[0] - window[1]).abs();
                if interval < 3600 && interval > 60 {
                    quick_cycles += 1;
                }
            }
        }

        if quick_cycles > signatures.len() / 3 {
            return Some(TradingPattern {
                pattern_type: PatternType::Sniper,
                confidence: 0.75,
                occurrences: quick_cycles,
                description: "Sniper pattern detected. Quick buy-sell cycles observed.".to_string(),
                risk_level: RiskLevel::High,
            });
        }

        None
    }

    fn detect_dca_pattern(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> Option<TradingPattern> {
        if signatures.len() < 10 {
            return None;
        }

        let timestamps: Vec<i64> = signatures
            .iter()
            .filter_map(|s| s.block_time)
            .collect();

        if timestamps.len() < 10 {
            return None;
        }

        // Calculate intervals
        let intervals: Vec<i64> = timestamps
            .windows(2)
            .map(|w| (w[0] - w[1]).abs())
            .collect();

        let avg = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
        let variance: f64 = intervals
            .iter()
            .map(|&i| {
                let diff = i as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / intervals.len() as f64;

        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / avg;

        // Regular intervals suggest DCA (low variance)
        if coefficient_of_variation < 0.3 && avg > 3600.0 {
            let interval_hours = avg / 3600.0;
            return Some(TradingPattern {
                pattern_type: PatternType::DcaBuyer,
                confidence: 0.8,
                occurrences: signatures.len(),
                description: format!(
                    "DCA pattern detected. Regular purchases approximately every {:.1} hours.",
                    interval_hours
                ),
                risk_level: RiskLevel::None,
            });
        }

        None
    }
}

/// Quick check if transaction timing suggests bot activity.
pub fn is_bot_timing(intervals_seconds: &[i64]) -> bool {
    if intervals_seconds.len() < 5 {
        return false;
    }

    let avg = intervals_seconds.iter().sum::<i64>() as f64 / intervals_seconds.len() as f64;
    
    // Sub-second or very regular timing suggests bot
    avg < 2.0 || {
        let variance: f64 = intervals_seconds
            .iter()
            .map(|&i| (i as f64 - avg).powi(2))
            .sum::<f64>() / intervals_seconds.len() as f64;
        variance.sqrt() < 1.0
    }
}

/// Detect if wallet might be wash trading.
pub fn detect_wash_trading(
    transactions: &[(Pubkey, Pubkey, u64)], // (from, to, amount)
) -> Option<TradingPattern> {
    let mut flow_map: HashMap<(Pubkey, Pubkey), u64> = HashMap::new();

    for (from, to, amount) in transactions {
        *flow_map.entry((*from, *to)).or_insert(0) += amount;
    }

    // Check for circular flows
    let mut circular_count = 0;
    for ((from, to), amount) in &flow_map {
        if let Some(reverse_amount) = flow_map.get(&(*to, *from)) {
            let ratio = *amount.min(reverse_amount) as f64 / *amount.max(reverse_amount) as f64;
            if ratio > 0.8 {
                circular_count += 1;
            }
        }
    }

    if circular_count > 2 {
        return Some(TradingPattern {
            pattern_type: PatternType::WashTrading,
            confidence: 0.7,
            occurrences: circular_count,
            description: "Potential wash trading detected. Circular fund flows observed.".to_string(),
            risk_level: RiskLevel::High,
        });
    }

    None
}
