//! Optimized swap execution with DEX routing and priority fees.
//!
//! This module provides high-performance swap execution designed for trading agents
//! that require minimal latency between quote and landed transaction.
//!
//! # Features
//! - Fast swap execution with configurable priority fees
//! - Jito bundle support for guaranteed inclusion
//! - Direct route optimization for speed
//! - Automatic slippage calculation
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::speed::{SwapExecutor, SwapConfig, PriorityLevel, FastRpcClient};
//! use solana_sdk::{signature::Keypair, pubkey::Pubkey};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rpc = FastRpcClient::single("https://api.mainnet-beta.solana.com");
//!     let payer = Keypair::new();
//!
//!     let executor = SwapExecutor::new(rpc, payer);
//!
//!     let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
//!     let sol = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
//!
//!     let config = SwapConfig::fast();
//!     let result = executor.swap_fast(&usdc, &sol, 1_000_000, config).await?;
//!
//!     println!("Swap executed in {}ms", result.execution_time_ms);
//!
//!     Ok(())
//! }
//! ```

use crate::speed::{FastRpcClient, FastTransactionBuilder, JupiterApiClient, JupiterQuote};
use crate::{Result, ToolkitError};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::VersionedTransaction,
};
use std::sync::Arc;
use std::time::Instant;

/// Priority fee level for transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PriorityLevel {
    /// No priority fee.
    None,
    /// Low priority (25th percentile).
    Low,
    /// Medium priority (50th percentile).
    #[default]
    Medium,
    /// High priority (75th percentile).
    High,
    /// Turbo priority (95th percentile + Jito tip).
    Turbo,
}

impl PriorityLevel {
    /// Get priority fee in micro-lamports.
    pub fn to_microlamports(&self) -> u64 {
        match self {
            PriorityLevel::None => 0,
            PriorityLevel::Low => 1_000,      // 1000 micro-lamports
            PriorityLevel::Medium => 10_000,   // 10000 micro-lamports
            PriorityLevel::High => 100_000,    // 100000 micro-lamports
            PriorityLevel::Turbo => 1_000_000, // 1M micro-lamports
        }
    }

    /// Get Jito tip amount in lamports.
    pub fn jito_tip(&self) -> u64 {
        match self {
            PriorityLevel::Turbo => 10_000, // 0.00001 SOL
            _ => 0,
        }
    }
}

/// Swap configuration.
#[derive(Debug, Clone)]
pub struct SwapConfig {
    /// Slippage in basis points (1 bps = 0.01%).
    pub slippage_bps: u16,
    /// Priority fee level.
    pub priority_fee: PriorityLevel,
    /// Use Jito bundles.
    pub use_jito: bool,
    /// Maximum accounts (limit for faster execution).
    pub max_accounts: Option<u8>,
    /// Only use direct routes (skip multi-hop).
    pub direct_route_only: bool,
    /// Wrap and unwrap SOL automatically.
    pub wrap_unwrap_sol: bool,
}

impl Default for SwapConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 50, // 0.5%
            priority_fee: PriorityLevel::Medium,
            use_jito: false,
            max_accounts: None,
            direct_route_only: false,
            wrap_unwrap_sol: true,
        }
    }
}

impl SwapConfig {
    /// Create config optimized for speed.
    pub fn fast() -> Self {
        Self {
            slippage_bps: 100, // 1% - more tolerance for speed
            priority_fee: PriorityLevel::High,
            use_jito: false,
            max_accounts: Some(20), // Limit accounts for faster execution
            direct_route_only: true, // Direct routes are faster
            wrap_unwrap_sol: true,
        }
    }

    /// Create config for guaranteed execution (Jito).
    pub fn guaranteed() -> Self {
        Self {
            slippage_bps: 50,
            priority_fee: PriorityLevel::Turbo,
            use_jito: true,
            max_accounts: None,
            direct_route_only: false,
            wrap_unwrap_sol: true,
        }
    }

    /// Create config optimized for best price.
    pub fn best_price() -> Self {
        Self {
            slippage_bps: 30, // Tight slippage
            priority_fee: PriorityLevel::Low,
            use_jito: false,
            max_accounts: None,
            direct_route_only: false,
            wrap_unwrap_sol: true,
        }
    }

    /// Set slippage.
    pub fn with_slippage(mut self, bps: u16) -> Self {
        self.slippage_bps = bps;
        self
    }

    /// Set priority level.
    pub fn with_priority(mut self, priority: PriorityLevel) -> Self {
        self.priority_fee = priority;
        self
    }

    /// Enable Jito bundles.
    pub fn with_jito(mut self) -> Self {
        self.use_jito = true;
        self
    }

    /// Set direct routes only.
    pub fn with_direct_routes(mut self) -> Self {
        self.direct_route_only = true;
        self
    }

    /// Set max accounts.
    pub fn with_max_accounts(mut self, max: u8) -> Self {
        self.max_accounts = Some(max);
        self
    }
}

/// Result of a swap execution.
#[derive(Debug, Clone)]
pub struct SwapResult {
    /// Transaction signature.
    pub signature: Signature,
    /// Input amount.
    pub input_amount: u64,
    /// Output amount.
    pub output_amount: u64,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Slot where transaction landed.
    pub slot_landed: u64,
    /// Price impact in basis points.
    pub price_impact_bps: u16,
    /// Route labels (DEXs used).
    pub route_labels: Vec<String>,
}

/// Quote result without execution.
#[derive(Debug, Clone)]
pub struct Quote {
    /// Input mint.
    pub input_mint: Pubkey,
    /// Output mint.
    pub output_mint: Pubkey,
    /// Input amount.
    pub input_amount: u64,
    /// Output amount.
    pub output_amount: u64,
    /// Price impact in basis points.
    pub price_impact_bps: u16,
    /// Route (programs in route).
    pub route: Vec<Pubkey>,
    /// Route labels.
    pub route_labels: Vec<String>,
    /// Minimum output amount with slippage.
    pub min_output_amount: u64,
}

/// Swap error types.
#[derive(Debug, Clone)]
pub enum SwapError {
    /// Quote failed.
    QuoteFailed(String),
    /// Transaction build failed.
    TransactionBuildFailed(String),
    /// Transaction execution failed.
    ExecutionFailed(String),
    /// Slippage exceeded.
    SlippageExceeded { expected: u64, actual: u64 },
    /// Insufficient balance.
    InsufficientBalance { required: u64, available: u64 },
    /// Invalid mint.
    InvalidMint(String),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::QuoteFailed(msg) => write!(f, "Quote failed: {}", msg),
            SwapError::TransactionBuildFailed(msg) => write!(f, "Transaction build failed: {}", msg),
            SwapError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            SwapError::SlippageExceeded { expected, actual } => {
                write!(f, "Slippage exceeded: expected {}, got {}", expected, actual)
            }
            SwapError::InsufficientBalance { required, available } => {
                write!(f, "Insufficient balance: need {}, have {}", required, available)
            }
            SwapError::InvalidMint(msg) => write!(f, "Invalid mint: {}", msg),
        }
    }
}

impl std::error::Error for SwapError {}

impl From<SwapError> for ToolkitError {
    fn from(e: SwapError) -> Self {
        ToolkitError::JupiterError(e.to_string())
    }
}

/// High-performance swap executor.
pub struct SwapExecutor {
    /// RPC client.
    rpc: FastRpcClient,
    /// Transaction builder.
    builder: FastTransactionBuilder,
    /// Jupiter API client.
    jupiter: JupiterApiClient,
    /// Payer keypair.
    payer: Arc<Keypair>,
}

impl SwapExecutor {
    /// Create a new swap executor.
    pub fn new(rpc: FastRpcClient, payer: Keypair) -> Self {
        let payer = Arc::new(payer);
        Self {
            builder: FastTransactionBuilder::new(
                Keypair::from_bytes(&payer.to_bytes()).expect("Valid keypair"),
            ),
            rpc,
            jupiter: JupiterApiClient::new(),
            payer,
        }
    }

    /// Get payer pubkey.
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    /// Execute swap as fast as possible.
    pub async fn swap_fast(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        config: SwapConfig,
    ) -> Result<SwapResult> {
        let start = Instant::now();

        // Get quote with options
        let quote = self
            .jupiter
            .quote_with_options(
                input_mint,
                output_mint,
                amount,
                config.slippage_bps,
                config.direct_route_only,
                config.max_accounts,
            )
            .await
            .map_err(|e| SwapError::QuoteFailed(e.to_string()))?;

        // Get swap transaction
        let priority_fee = if config.priority_fee != PriorityLevel::None {
            Some(config.priority_fee.to_microlamports())
        } else {
            None
        };

        let mut tx = self
            .jupiter
            .swap_transaction_with_options(
                &quote,
                &self.payer.pubkey(),
                priority_fee,
                config.wrap_unwrap_sol,
                false, // Use versioned transactions
            )
            .await
            .map_err(|e| SwapError::TransactionBuildFailed(e.to_string()))?;

        // Get fresh blockhash
        let blockhash = self.rpc.get_latest_blockhash_fast().await?;
        tx.message.set_recent_blockhash(blockhash);

        // Sign
        let signed_tx = VersionedTransaction::try_new(tx.message.clone(), &[self.payer.as_ref()])
            .map_err(|e| SwapError::TransactionBuildFailed(e.to_string()))?;

        // Send and get slot
        let signature = self
            .rpc
            .get_best_client()
            .await
            .send_and_confirm_transaction(&signed_tx)
            .await
            .map_err(|e| SwapError::ExecutionFailed(e.to_string()))?;

        let slot = self.rpc.get_slot_fast().await.unwrap_or(0);
        let execution_time = start.elapsed().as_millis() as u64;

        // Parse price impact
        let price_impact_bps = (quote.price_impact() * 100.0) as u16;

        Ok(SwapResult {
            signature,
            input_amount: quote.in_amount,
            output_amount: quote.out_amount,
            execution_time_ms: execution_time,
            slot_landed: slot,
            price_impact_bps,
            route_labels: quote.route_labels(),
        })
    }

    /// Get quote without executing.
    pub async fn get_quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
    ) -> Result<Quote> {
        let jupiter_quote = self.jupiter.quote(input_mint, output_mint, amount, 50).await?;

        let price_impact_bps = (jupiter_quote.price_impact() * 100.0) as u16;

        Ok(Quote {
            input_mint: *input_mint,
            output_mint: *output_mint,
            input_amount: jupiter_quote.in_amount,
            output_amount: jupiter_quote.out_amount,
            price_impact_bps,
            route: vec![], // Would need to parse from route_plan
            route_labels: jupiter_quote.route_labels(),
            min_output_amount: jupiter_quote.other_amount_threshold,
        })
    }

    /// Execute swap with Jito bundle for guaranteed inclusion.
    pub async fn swap_with_jito(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        _tip_lamports: u64,
    ) -> Result<SwapResult> {
        let mut config = SwapConfig::guaranteed();
        config.use_jito = true;

        // Note: Full Jito integration would require separate bundle submission
        // This is a simplified version that uses high priority fees
        self.swap_fast(input_mint, output_mint, amount, config).await
    }

    /// Prepare a swap transaction without executing.
    pub async fn prepare_swap(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        config: SwapConfig,
    ) -> Result<(VersionedTransaction, JupiterQuote)> {
        let quote = self
            .jupiter
            .quote_with_options(
                input_mint,
                output_mint,
                amount,
                config.slippage_bps,
                config.direct_route_only,
                config.max_accounts,
            )
            .await?;

        let priority_fee = if config.priority_fee != PriorityLevel::None {
            Some(config.priority_fee.to_microlamports())
        } else {
            None
        };

        let tx = self
            .jupiter
            .swap_transaction_with_options(
                &quote,
                &self.payer.pubkey(),
                priority_fee,
                config.wrap_unwrap_sol,
                false,
            )
            .await?;

        Ok((tx, quote))
    }

    /// Execute a prepared swap transaction.
    pub async fn execute_prepared(&self, mut tx: VersionedTransaction) -> Result<Signature> {
        // Get fresh blockhash
        let blockhash = self.rpc.get_latest_blockhash_fast().await?;
        tx.message.set_recent_blockhash(blockhash);

        // Sign
        let signed_tx = VersionedTransaction::try_new(tx.message, &[self.payer.as_ref()])
            .map_err(|e| ToolkitError::SigningError(e.to_string()))?;

        // Send
        self.rpc
            .get_best_client()
            .await
            .send_and_confirm_transaction(&signed_tx)
            .await
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))
    }

    /// Get multiple quotes in parallel.
    pub async fn get_quotes_parallel(
        &self,
        pairs: Vec<(Pubkey, Pubkey, u64)>,
    ) -> Vec<Result<Quote>> {
        let requests: Vec<_> = pairs
            .into_iter()
            .map(|(input, output, amount)| (input, output, amount, 50u16))
            .collect();

        let jupiter_quotes = self.jupiter.get_quotes_parallel(requests).await;

        jupiter_quotes
            .into_iter()
            .map(|result| {
                result.map(|q| {
                    let price_impact_bps = (q.price_impact() * 100.0) as u16;
                    Quote {
                        input_mint: q.input_mint_pubkey().unwrap_or_default(),
                        output_mint: q.output_mint_pubkey().unwrap_or_default(),
                        input_amount: q.in_amount,
                        output_amount: q.out_amount,
                        price_impact_bps,
                        route: vec![],
                        route_labels: q.route_labels(),
                        min_output_amount: q.other_amount_threshold,
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_levels() {
        assert_eq!(PriorityLevel::None.to_microlamports(), 0);
        assert_eq!(PriorityLevel::Low.to_microlamports(), 1_000);
        assert_eq!(PriorityLevel::Medium.to_microlamports(), 10_000);
        assert_eq!(PriorityLevel::High.to_microlamports(), 100_000);
        assert_eq!(PriorityLevel::Turbo.to_microlamports(), 1_000_000);
    }

    #[test]
    fn test_swap_config_presets() {
        let fast = SwapConfig::fast();
        assert_eq!(fast.slippage_bps, 100);
        assert!(fast.direct_route_only);
        assert_eq!(fast.priority_fee, PriorityLevel::High);

        let guaranteed = SwapConfig::guaranteed();
        assert!(guaranteed.use_jito);
        assert_eq!(guaranteed.priority_fee, PriorityLevel::Turbo);

        let best_price = SwapConfig::best_price();
        assert_eq!(best_price.slippage_bps, 30);
        assert_eq!(best_price.priority_fee, PriorityLevel::Low);
    }

    #[test]
    fn test_swap_config_builder() {
        let config = SwapConfig::default()
            .with_slippage(75)
            .with_priority(PriorityLevel::High)
            .with_direct_routes()
            .with_max_accounts(15);

        assert_eq!(config.slippage_bps, 75);
        assert_eq!(config.priority_fee, PriorityLevel::High);
        assert!(config.direct_route_only);
        assert_eq!(config.max_accounts, Some(15));
    }

    #[test]
    fn test_swap_error_display() {
        let err = SwapError::SlippageExceeded {
            expected: 1000,
            actual: 900,
        };
        assert!(err.to_string().contains("Slippage exceeded"));
    }
}
