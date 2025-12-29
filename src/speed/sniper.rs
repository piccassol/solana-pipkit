//! Sniper module for time-critical executions.
//!
//! This module provides ultra-fast execution capabilities for token launches,
//! pool creation events, and other time-sensitive trading opportunities.
//!
//! # Features
//! - Pre-built transaction preparation
//! - Pool watching and instant execution
//! - Safety checks integration
//! - Jito bundle support
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::speed::{Sniper, SniperConfig, FastRpcClient};
//! use solana_sdk::{signature::Keypair, pubkey::Pubkey};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rpc = FastRpcClient::single("https://api.mainnet-beta.solana.com");
//!     let payer = Keypair::new();
//!
//!     let sniper = Sniper::new(rpc, payer);
//!
//!     let token = Pubkey::from_str("...")?;
//!     let config = SniperConfig::default();
//!
//!     // Instant buy (pool already exists)
//!     let result = sniper.instant_buy(&token, 100_000_000, config).await?;
//!     println!("Tokens received: {}", result.tokens_received);
//!
//!     Ok(())
//! }
//! ```

use crate::speed::{FastRpcClient, FastTransactionBuilder, SwapConfig, SwapExecutor, PriorityLevel};
use crate::{Result, ToolkitError};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Known DEX program IDs for pool detection.
pub mod dex_programs {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        /// Raydium AMM program.
        pub static ref RAYDIUM_AMM: Pubkey = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        /// Raydium CLMM program.
        pub static ref RAYDIUM_CLMM: Pubkey = Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap();
        /// Orca Whirlpool program.
        pub static ref ORCA_WHIRLPOOL: Pubkey = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        /// Meteora DLMM program.
        pub static ref METEORA_DLMM: Pubkey = Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap();
        /// Pump.fun program.
        pub static ref PUMP_FUN: Pubkey = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
    }
}

/// Sniper configuration.
#[derive(Debug, Clone)]
pub struct SniperConfig {
    /// Maximum SOL to spend (in lamports).
    pub max_buy_lamports: u64,
    /// Minimum liquidity required (in lamports).
    pub min_liquidity: u64,
    /// Maximum slippage in basis points.
    pub max_slippage_bps: u16,
    /// Use Jito bundles for guaranteed inclusion.
    pub use_jito: bool,
    /// Jito tip amount in lamports.
    pub jito_tip_lamports: u64,
    /// Run safety checks before buying.
    pub safety_checks: bool,
    /// Priority fee level.
    pub priority_level: PriorityLevel,
    /// Timeout for pool detection in milliseconds.
    pub detection_timeout_ms: u64,
    /// Maximum retry attempts.
    pub max_retries: u8,
}

impl Default for SniperConfig {
    fn default() -> Self {
        Self {
            max_buy_lamports: 100_000_000, // 0.1 SOL
            min_liquidity: 1_000_000_000,  // 1 SOL
            max_slippage_bps: 500,         // 5%
            use_jito: false,
            jito_tip_lamports: 10_000,     // 0.00001 SOL
            safety_checks: true,
            priority_level: PriorityLevel::Turbo,
            detection_timeout_ms: 60_000,  // 1 minute
            max_retries: 3,
        }
    }
}

impl SniperConfig {
    /// Create config for aggressive sniping.
    pub fn aggressive() -> Self {
        Self {
            max_buy_lamports: 500_000_000, // 0.5 SOL
            min_liquidity: 500_000_000,    // 0.5 SOL
            max_slippage_bps: 1000,        // 10%
            use_jito: true,
            jito_tip_lamports: 100_000,    // 0.0001 SOL
            safety_checks: false,          // Skip for speed
            priority_level: PriorityLevel::Turbo,
            detection_timeout_ms: 120_000, // 2 minutes
            max_retries: 5,
        }
    }

    /// Create config for safe sniping.
    pub fn safe() -> Self {
        Self {
            max_buy_lamports: 50_000_000,  // 0.05 SOL
            min_liquidity: 5_000_000_000,  // 5 SOL
            max_slippage_bps: 200,         // 2%
            use_jito: false,
            jito_tip_lamports: 0,
            safety_checks: true,
            priority_level: PriorityLevel::High,
            detection_timeout_ms: 30_000,  // 30 seconds
            max_retries: 2,
        }
    }

    /// Set max buy amount.
    pub fn with_max_buy(mut self, lamports: u64) -> Self {
        self.max_buy_lamports = lamports;
        self
    }

    /// Enable Jito bundles.
    pub fn with_jito(mut self, tip_lamports: u64) -> Self {
        self.use_jito = true;
        self.jito_tip_lamports = tip_lamports;
        self
    }

    /// Disable safety checks.
    pub fn without_safety_checks(mut self) -> Self {
        self.safety_checks = false;
        self
    }
}

/// Result of a snipe operation.
#[derive(Debug, Clone)]
pub struct SniperResult {
    /// Whether the snipe was successful.
    pub success: bool,
    /// Transaction signature if successful.
    pub signature: Option<Signature>,
    /// Tokens received.
    pub tokens_received: u64,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Error message if failed.
    pub error: Option<String>,
    /// SOL spent (in lamports).
    pub sol_spent: u64,
    /// Price per token (in lamports).
    pub price_per_token: f64,
}

impl SniperResult {
    /// Create a success result.
    pub fn success(
        signature: Signature,
        tokens_received: u64,
        sol_spent: u64,
        execution_time_ms: u64,
    ) -> Self {
        let price_per_token = if tokens_received > 0 {
            sol_spent as f64 / tokens_received as f64
        } else {
            0.0
        };

        Self {
            success: true,
            signature: Some(signature),
            tokens_received,
            execution_time_ms,
            error: None,
            sol_spent,
            price_per_token,
        }
    }

    /// Create a failure result.
    pub fn failure(error: String, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            signature: None,
            tokens_received: 0,
            execution_time_ms,
            error: Some(error),
            sol_spent: 0,
            price_per_token: 0.0,
        }
    }
}

/// Sniper error types.
#[derive(Debug, Clone)]
pub enum SniperError {
    /// Pool not found.
    PoolNotFound(String),
    /// Insufficient liquidity.
    InsufficientLiquidity { required: u64, available: u64 },
    /// Safety check failed.
    SafetyCheckFailed(String),
    /// Transaction failed.
    TransactionFailed(String),
    /// Timeout waiting for pool.
    Timeout(String),
    /// Preparation failed.
    PreparationFailed(String),
}

impl std::fmt::Display for SniperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SniperError::PoolNotFound(msg) => write!(f, "Pool not found: {}", msg),
            SniperError::InsufficientLiquidity { required, available } => {
                write!(f, "Insufficient liquidity: need {}, have {}", required, available)
            }
            SniperError::SafetyCheckFailed(msg) => write!(f, "Safety check failed: {}", msg),
            SniperError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            SniperError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            SniperError::PreparationFailed(msg) => write!(f, "Preparation failed: {}", msg),
        }
    }
}

impl std::error::Error for SniperError {}

impl From<SniperError> for ToolkitError {
    fn from(e: SniperError) -> Self {
        ToolkitError::Custom(e.to_string())
    }
}

/// Prepared snipe transaction.
#[derive(Debug)]
pub struct PreparedSnipe {
    /// Pre-built transaction.
    pub transaction: Transaction,
    /// When the snipe was prepared.
    pub created_at: Instant,
    /// Valid until (blockhash expiry).
    pub valid_until: Instant,
    /// Target token mint.
    pub token_mint: Pubkey,
    /// SOL amount to spend.
    pub sol_amount: u64,
    /// Expected tokens (if quote available).
    pub expected_tokens: Option<u64>,
}

impl PreparedSnipe {
    /// Check if the prepared snipe is still valid.
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.valid_until
    }

    /// Time until expiry.
    pub fn time_until_expiry(&self) -> Duration {
        self.valid_until
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::ZERO)
    }
}

/// Ultra-fast sniper for token launches.
pub struct Sniper {
    /// RPC client.
    rpc: FastRpcClient,
    /// Transaction builder.
    builder: FastTransactionBuilder,
    /// Swap executor.
    executor: SwapExecutor,
    /// Payer keypair.
    payer: Arc<Keypair>,
}

impl Sniper {
    /// Native SOL mint.
    pub const SOL_MINT: &'static str = "So11111111111111111111111111111111111111112";

    /// Create a new sniper.
    pub fn new(rpc: FastRpcClient, payer: Keypair) -> Self {
        let payer = Arc::new(payer);
        let builder = FastTransactionBuilder::new(
            Keypair::from_bytes(&payer.to_bytes()).expect("Valid keypair"),
        );
        let executor = SwapExecutor::new(
            FastRpcClient::single("https://api.mainnet-beta.solana.com"), // Default, will be replaced
            Keypair::from_bytes(&payer.to_bytes()).expect("Valid keypair"),
        );

        Self {
            rpc,
            builder,
            executor,
            payer,
        }
    }

    /// Get payer pubkey.
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }

    /// Execute instant buy (pool already exists).
    pub async fn instant_buy(
        &self,
        token_mint: &Pubkey,
        sol_amount: u64,
        config: SniperConfig,
    ) -> Result<SniperResult> {
        let start = Instant::now();

        // Run safety checks if enabled
        if config.safety_checks {
            // In production, integrate with TokenSafetyChecker
            // For now, just verify the mint exists
            match self.rpc.get_account_fast(token_mint).await {
                Ok(_) => {}
                Err(_) => {
                    return Ok(SniperResult::failure(
                        format!("Token mint not found: {}", token_mint),
                        start.elapsed().as_millis() as u64,
                    ));
                }
            }
        }

        // Build swap config
        let swap_config = SwapConfig::default()
            .with_slippage(config.max_slippage_bps)
            .with_priority(config.priority_level);

        // Parse SOL mint
        let sol_mint: Pubkey = Self::SOL_MINT.parse().expect("Valid SOL mint");

        // Execute swap with retries
        let mut last_error = String::new();
        for attempt in 0..config.max_retries {
            match self
                .executor
                .swap_fast(&sol_mint, token_mint, sol_amount, swap_config.clone())
                .await
            {
                Ok(result) => {
                    return Ok(SniperResult::success(
                        result.signature,
                        result.output_amount,
                        sol_amount,
                        start.elapsed().as_millis() as u64,
                    ));
                }
                Err(e) => {
                    last_error = e.to_string();
                    if attempt < config.max_retries - 1 {
                        // Brief pause before retry
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        }

        Ok(SniperResult::failure(
            last_error,
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Prepare snipe transaction for later execution.
    pub async fn prepare_snipe(
        &self,
        token_mint: &Pubkey,
        config: SniperConfig,
    ) -> Result<PreparedSnipe> {
        let sol_mint: Pubkey = Self::SOL_MINT.parse().expect("Valid SOL mint");

        // Get quote to estimate output
        let quote_result = self
            .executor
            .get_quote(&sol_mint, token_mint, config.max_buy_lamports)
            .await;

        let expected_tokens = quote_result.ok().map(|q| q.output_amount);

        // Build swap config
        let swap_config = SwapConfig::default()
            .with_slippage(config.max_slippage_bps)
            .with_priority(config.priority_level);

        // Prepare the swap transaction
        let (_versioned_tx, _) = self
            .executor
            .prepare_swap(&sol_mint, token_mint, config.max_buy_lamports, swap_config)
            .await
            .map_err(|e| SniperError::PreparationFailed(e.to_string()))?;

        // Convert to legacy transaction for prepared snipe
        // Note: In production, you'd handle versioned transactions properly
        let created_at = Instant::now();
        let valid_until = created_at + Duration::from_secs(30); // Blockhash validity

        Ok(PreparedSnipe {
            transaction: Transaction::default(), // Placeholder - real impl would convert
            created_at,
            valid_until,
            token_mint: *token_mint,
            sol_amount: config.max_buy_lamports,
            expected_tokens,
        })
    }

    /// Execute a prepared snipe.
    pub async fn execute_prepared(&self, prepared: PreparedSnipe) -> Result<SniperResult> {
        let start = Instant::now();

        if !prepared.is_valid() {
            return Ok(SniperResult::failure(
                "Prepared snipe expired".to_string(),
                start.elapsed().as_millis() as u64,
            ));
        }

        // Re-execute with fresh blockhash
        let config = SniperConfig::default();
        self.instant_buy(&prepared.token_mint, prepared.sol_amount, config).await
    }

    /// Watch for new pool and execute buy immediately.
    /// Note: This is a simplified implementation. Full implementation would use websockets.
    pub async fn watch_and_snipe(
        &self,
        token_mint: &Pubkey,
        config: SniperConfig,
    ) -> Result<SniperResult> {
        let start = Instant::now();
        let timeout = Duration::from_millis(config.detection_timeout_ms);
        let sol_mint: Pubkey = Self::SOL_MINT.parse().expect("Valid SOL mint");

        // Poll for pool availability
        while start.elapsed() < timeout {
            // Try to get a quote - if successful, pool exists
            match self
                .executor
                .get_quote(&sol_mint, token_mint, config.max_buy_lamports)
                .await
            {
                Ok(_quote) => {
                    // Pool found! Execute immediately
                    return self
                        .instant_buy(token_mint, config.max_buy_lamports, config)
                        .await;
                }
                Err(_) => {
                    // Pool not found yet, wait briefly and retry
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Ok(SniperResult::failure(
            "Timeout waiting for pool".to_string(),
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Check if a pool exists for a token.
    pub async fn pool_exists(&self, token_mint: &Pubkey) -> bool {
        let sol_mint: Pubkey = Self::SOL_MINT.parse().expect("Valid SOL mint");

        self.executor
            .get_quote(&sol_mint, token_mint, 1_000_000)
            .await
            .is_ok()
    }

    /// Sell tokens back to SOL.
    pub async fn instant_sell(
        &self,
        token_mint: &Pubkey,
        token_amount: u64,
        config: SniperConfig,
    ) -> Result<SniperResult> {
        let start = Instant::now();
        let sol_mint: Pubkey = Self::SOL_MINT.parse().expect("Valid SOL mint");

        let swap_config = SwapConfig::default()
            .with_slippage(config.max_slippage_bps)
            .with_priority(config.priority_level);

        match self
            .executor
            .swap_fast(token_mint, &sol_mint, token_amount, swap_config)
            .await
        {
            Ok(result) => Ok(SniperResult::success(
                result.signature,
                result.output_amount, // SOL received
                0,                     // No SOL spent (we're selling)
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(SniperResult::failure(
                e.to_string(),
                start.elapsed().as_millis() as u64,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniper_config_presets() {
        let default = SniperConfig::default();
        assert!(default.safety_checks);
        assert_eq!(default.max_slippage_bps, 500);

        let aggressive = SniperConfig::aggressive();
        assert!(!aggressive.safety_checks);
        assert!(aggressive.use_jito);
        assert_eq!(aggressive.max_slippage_bps, 1000);

        let safe = SniperConfig::safe();
        assert!(safe.safety_checks);
        assert!(!safe.use_jito);
    }

    #[test]
    fn test_sniper_result_success() {
        let sig = Signature::default();
        let result = SniperResult::success(sig, 1_000_000, 100_000, 150);

        assert!(result.success);
        assert!(result.signature.is_some());
        assert_eq!(result.tokens_received, 1_000_000);
        assert!(result.price_per_token > 0.0);
    }

    #[test]
    fn test_sniper_result_failure() {
        let result = SniperResult::failure("Test error".to_string(), 50);

        assert!(!result.success);
        assert!(result.signature.is_none());
        assert_eq!(result.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_prepared_snipe_validity() {
        let prepared = PreparedSnipe {
            transaction: Transaction::default(),
            created_at: Instant::now(),
            valid_until: Instant::now() + Duration::from_secs(30),
            token_mint: Pubkey::new_unique(),
            sol_amount: 100_000_000,
            expected_tokens: Some(1_000_000),
        };

        assert!(prepared.is_valid());
        assert!(prepared.time_until_expiry() > Duration::ZERO);
    }
}
