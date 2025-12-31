//! Wallet profiling and classification.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use std::collections::HashSet;
use crate::{Result, ToolkitError};
use super::AnalyticsConfig;

/// Known smart money addresses (exchanges, funds, notable traders).
/// In production, this would be loaded from a maintained registry.
static KNOWN_EXCHANGES: &[&str] = &[
    "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM", // Binance
    "2ojv9BAiHUrvsm9gxDe7fJSzbNZSJcxZvf8dqmWGHG8S", // Coinbase
];

static KNOWN_SMART_MONEY: &[&str] = &[
    // Placeholder for known profitable traders, funds, etc.
];

#[derive(Debug, Clone)]
pub struct WalletProfile {
    pub address: Pubkey,
    pub sol_balance: f64,
    pub token_count: usize,
    pub classification: WalletClassification,
    pub activity_level: ActivityLevel,
    pub flags: Vec<WalletFlag>,
    pub first_seen: Option<i64>,
    pub transaction_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WalletClassification {
    Whale,
    SmartMoney,
    Exchange,
    Bot,
    Retail,
    New,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivityLevel {
    VeryHigh,  // 50+ txs/day
    High,      // 10-50 txs/day
    Medium,    // 1-10 txs/day
    Low,       // < 1 tx/day
    Dormant,   // No activity in 30+ days
}

#[derive(Debug, Clone)]
pub enum WalletFlag {
    HighValueHolder,
    FrequentTrader,
    LiquidityProvider,
    NftCollector,
    DeFiUser,
    NewWallet,
    PossibleBot,
    KnownEntity(String),
}

pub struct WalletProfiler<'a> {
    client: &'a RpcClient,
    config: AnalyticsConfig,
    exchange_set: HashSet<Pubkey>,
    smart_money_set: HashSet<Pubkey>,
}

impl<'a> WalletProfiler<'a> {
    pub fn new(client: &'a RpcClient, config: AnalyticsConfig) -> Self {
        let exchange_set: HashSet<Pubkey> = KNOWN_EXCHANGES
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        let smart_money_set: HashSet<Pubkey> = KNOWN_SMART_MONEY
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        Self {
            client,
            config,
            exchange_set,
            smart_money_set,
        }
    }

    pub async fn analyze(&self, wallet: &Pubkey) -> Result<WalletProfile> {
        let balance = self.client
            .get_balance(wallet)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let sol_balance = balance as f64 / LAMPORTS_PER_SOL as f64;

        let token_accounts = self.client
            .get_token_accounts_by_owner(
                wallet,
                solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id()),
            )
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let token_count = token_accounts.len();

        let signatures = self.client
            .get_signatures_for_address(wallet)
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        let transaction_count = signatures.len() as u64;
        let first_seen = signatures.last().and_then(|s| s.block_time);

        let activity_level = self.calculate_activity_level(&signatures);
        let classification = self.classify_wallet(wallet, sol_balance, &activity_level, transaction_count);
        let flags = self.detect_flags(wallet, sol_balance, token_count, &activity_level);

        Ok(WalletProfile {
            address: *wallet,
            sol_balance,
            token_count,
            classification,
            activity_level,
            flags,
            first_seen,
            transaction_count,
        })
    }

    fn calculate_activity_level(
        &self,
        signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    ) -> ActivityLevel {
        if signatures.is_empty() {
            return ActivityLevel::Dormant;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let recent_count = signatures
            .iter()
            .filter(|s| {
                s.block_time
                    .map(|t| now - t < 86400) // Last 24 hours
                    .unwrap_or(false)
            })
            .count();

        let last_activity = signatures.first().and_then(|s| s.block_time).unwrap_or(0);
        let days_since_last = (now - last_activity) / 86400;

        if days_since_last > 30 {
            ActivityLevel::Dormant
        } else if recent_count >= 50 {
            ActivityLevel::VeryHigh
        } else if recent_count >= 10 {
            ActivityLevel::High
        } else if recent_count >= 1 {
            ActivityLevel::Medium
        } else {
            ActivityLevel::Low
        }
    }

    fn classify_wallet(
        &self,
        wallet: &Pubkey,
        sol_balance: f64,
        activity: &ActivityLevel,
        tx_count: u64,
    ) -> WalletClassification {
        if self.exchange_set.contains(wallet) {
            return WalletClassification::Exchange;
        }

        if self.smart_money_set.contains(wallet) {
            return WalletClassification::SmartMoney;
        }

        if sol_balance >= self.config.whale_threshold_sol {
            return WalletClassification::Whale;
        }

        if tx_count < 10 {
            return WalletClassification::New;
        }

        // High activity with moderate balance might indicate a bot
        if matches!(activity, ActivityLevel::VeryHigh) && sol_balance < 100.0 {
            return WalletClassification::Bot;
        }

        WalletClassification::Retail
    }

    fn detect_flags(
        &self,
        wallet: &Pubkey,
        sol_balance: f64,
        token_count: usize,
        activity: &ActivityLevel,
    ) -> Vec<WalletFlag> {
        let mut flags = Vec::new();

        if sol_balance >= self.config.whale_threshold_sol {
            flags.push(WalletFlag::HighValueHolder);
        }

        if matches!(activity, ActivityLevel::VeryHigh | ActivityLevel::High) {
            flags.push(WalletFlag::FrequentTrader);
        }

        if token_count > 50 {
            flags.push(WalletFlag::DeFiUser);
        }

        if matches!(activity, ActivityLevel::VeryHigh) && sol_balance < 10.0 {
            flags.push(WalletFlag::PossibleBot);
        }

        if self.exchange_set.contains(wallet) {
            flags.push(WalletFlag::KnownEntity("Exchange".to_string()));
        }

        if self.smart_money_set.contains(wallet) {
            flags.push(WalletFlag::KnownEntity("Smart Money".to_string()));
        }

        flags
    }
}

/// Check if a wallet is a known whale.
pub fn is_whale(sol_balance: f64, threshold: f64) -> bool {
    sol_balance >= threshold
}

/// Check if address is a known exchange.
pub fn is_exchange(address: &Pubkey) -> bool {
    let addr_str = address.to_string();
    KNOWN_EXCHANGES.contains(&addr_str.as_str())
}

/// Check if address is known smart money.
pub fn is_smart_money(address: &Pubkey) -> bool {
    let addr_str = address.to_string();
    KNOWN_SMART_MONEY.contains(&addr_str.as_str())
}
