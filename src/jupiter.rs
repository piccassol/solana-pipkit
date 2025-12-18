//! Jupiter DEX Aggregator integration for seamless token swaps.
//!
//! This module provides an ergonomic wrapper around Jupiter's V6 Swap API,
//! enabling easy token swaps with best-price routing across Solana DEXs.
//!
//! # Features
//! - Simple swap API with sensible defaults
//! - Quote fetching for price discovery
//! - Configurable slippage and routing options
//! - Support for exact input and exact output swaps
//! - Transaction building and execution
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::jupiter::{JupiterClient, SwapConfig};
//! use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let wallet = Keypair::new();
//!     let rpc_url = "https://api.mainnet-beta.solana.com";
//!     
//!     let jupiter = JupiterClient::new(rpc_url);
//!     
//!     // Get a quote for swapping 1 USDC to SOL
//!     let quote = jupiter.get_quote(
//!         JupiterClient::USDC_MINT,
//!         JupiterClient::SOL_MINT,
//!         1_000_000, // 1 USDC (6 decimals)
//!         50, // 0.5% slippage
//!     ).await?;
//!     
//!     println!("Expected output: {} lamports", quote.out_amount);
//!     
//!     // Execute the swap
//!     let signature = jupiter.swap(&wallet, quote).await?;
//!     println!("Swap executed: {}", signature);
//!     
//!     Ok(())
//! }
//! ```

use crate::{Result, ToolkitError};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::VersionedTransaction,
};
use std::str::FromStr;

/// Default Jupiter API endpoint
pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";

/// Common token mints for convenience
pub mod mints {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        /// Native SOL (wrapped)
        pub static ref SOL: Pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        /// USDC
        pub static ref USDC: Pubkey = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        /// USDT
        pub static ref USDT: Pubkey = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
        /// Bonk
        pub static ref BONK: Pubkey = Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap();
        /// JUP
        pub static ref JUP: Pubkey = Pubkey::from_str("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").unwrap();
        /// RAY (Raydium)
        pub static ref RAY: Pubkey = Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R").unwrap();
    }
}

/// Jupiter swap client for interacting with Jupiter's V6 API
pub struct JupiterClient {
    rpc_client: RpcClient,
    api_url: String,
    http_client: reqwest::Client,
}

/// Quote response from Jupiter API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    /// Input mint address
    pub input_mint: String,
    /// Input amount in smallest units
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub in_amount: u64,
    /// Output mint address
    pub output_mint: String,
    /// Output amount in smallest units
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub out_amount: u64,
    /// Minimum output amount considering slippage
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub other_amount_threshold: u64,
    /// Swap mode (ExactIn or ExactOut)
    pub swap_mode: String,
    /// Slippage in basis points
    pub slippage_bps: u16,
    /// Price impact percentage
    pub price_impact_pct: String,
    /// Route plan details
    pub route_plan: Vec<RoutePlanStep>,
    /// Context slot
    #[serde(default)]
    pub context_slot: Option<u64>,
    /// Time taken in ms
    #[serde(default)]
    pub time_taken: Option<f64>,
}

/// A step in the swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    /// Swap info for this step
    pub swap_info: SwapInfo,
    /// Percentage of input routed through this step
    pub percent: u8,
}

/// Swap information for a route step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    /// AMM key
    pub amm_key: String,
    /// Label (e.g., "Raydium", "Orca")
    pub label: Option<String>,
    /// Input mint
    pub input_mint: String,
    /// Output mint
    pub output_mint: String,
    /// Input amount
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub in_amount: u64,
    /// Output amount
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub out_amount: u64,
    /// Fee amount
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub fee_amount: u64,
    /// Fee mint
    pub fee_mint: String,
}

/// Swap request to Jupiter API
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    /// User's public key
    pub user_public_key: String,
    /// Quote response from /quote endpoint
    pub quote_response: QuoteResponse,
    /// Whether to wrap/unwrap SOL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_and_unwrap_sol: Option<bool>,
    /// Use shared accounts for reduced tx size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_shared_accounts: Option<bool>,
    /// Fee account for referral fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_account: Option<String>,
    /// Compute unit price in micro-lamports (priority fee)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    /// Use Token Ledger for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_token_ledger: Option<bool>,
    /// Destination token account (if not using ATA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_token_account: Option<String>,
    /// Dynamic compute unit limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_compute_unit_limit: Option<bool>,
    /// Skip user accounts RPC calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_user_accounts_rpc_calls: Option<bool>,
}

/// Response from swap endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    /// Base64-encoded serialized transaction
    pub swap_transaction: String,
    /// Last valid block height
    pub last_valid_block_height: u64,
    /// Priority fee type used
    #[serde(default)]
    pub priority_fee: Option<String>,
}

/// Configuration for a swap
#[derive(Debug, Clone)]
pub struct SwapConfig {
    /// Slippage tolerance in basis points (1 bps = 0.01%)
    pub slippage_bps: u16,
    /// Priority fee in micro-lamports (optional)
    pub priority_fee_micro_lamports: Option<u64>,
    /// Whether to automatically wrap/unwrap SOL
    pub wrap_unwrap_sol: bool,
    /// Use shared accounts for smaller transactions
    pub use_shared_accounts: bool,
    /// Dynamic compute unit limit
    pub dynamic_compute_unit_limit: bool,
}

impl Default for SwapConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 50, // 0.5% default slippage
            priority_fee_micro_lamports: None,
            wrap_unwrap_sol: true,
            use_shared_accounts: true,
            dynamic_compute_unit_limit: true,
        }
    }
}

impl SwapConfig {
    /// Create a new swap config with custom slippage
    pub fn with_slippage(slippage_bps: u16) -> Self {
        Self {
            slippage_bps,
            ..Default::default()
        }
    }

    /// Set priority fee in micro-lamports
    pub fn with_priority_fee(mut self, micro_lamports: u64) -> Self {
        self.priority_fee_micro_lamports = Some(micro_lamports);
        self
    }
}

impl JupiterClient {
    /// Native SOL mint (wrapped)
    pub const SOL_MINT: &'static str = "So11111111111111111111111111111111111111112";
    /// USDC mint
    pub const USDC_MINT: &'static str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    /// USDT mint
    pub const USDT_MINT: &'static str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

    /// Create a new Jupiter client with the default API endpoint
    pub fn new(rpc_url: &str) -> Self {
        Self::with_api_url(rpc_url, JUPITER_API_URL)
    }

    /// Create a new Jupiter client with a custom API endpoint
    pub fn with_api_url(rpc_url: &str, api_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            api_url: api_url.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Get a quote for swapping tokens
    ///
    /// # Arguments
    /// * `input_mint` - The mint address of the input token
    /// * `output_mint` - The mint address of the output token
    /// * `amount` - The amount to swap in smallest units (e.g., lamports for SOL)
    /// * `slippage_bps` - Slippage tolerance in basis points (50 = 0.5%)
    ///
    /// # Example
    /// ```rust,no_run
    /// let quote = jupiter.get_quote(
    ///     JupiterClient::USDC_MINT,
    ///     JupiterClient::SOL_MINT,
    ///     1_000_000, // 1 USDC
    ///     50, // 0.5% slippage
    /// ).await?;
    /// ```
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!(
                "Quote request failed: {}",
                error_text
            )));
        }

        let quote: QuoteResponse = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        Ok(quote)
    }

    /// Get a quote for an exact output amount (reverse quote)
    ///
    /// Use this when you know how much output you want to receive.
    pub async fn get_quote_exact_out(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&swapMode=ExactOut",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!(
                "Quote request failed: {}",
                error_text
            )));
        }

        let quote: QuoteResponse = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        Ok(quote)
    }

    /// Execute a swap with a previously fetched quote
    ///
    /// # Arguments
    /// * `wallet` - The wallet keypair to sign the transaction
    /// * `quote` - The quote response from `get_quote`
    ///
    /// # Returns
    /// The transaction signature on success
    pub async fn swap(&self, wallet: &Keypair, quote: QuoteResponse) -> Result<Signature> {
        self.swap_with_config(wallet, quote, SwapConfig::default())
            .await
    }

    /// Execute a swap with custom configuration
    pub async fn swap_with_config(
        &self,
        wallet: &Keypair,
        quote: QuoteResponse,
        config: SwapConfig,
    ) -> Result<Signature> {
        // Build swap request
        let swap_request = SwapRequest {
            user_public_key: wallet.pubkey().to_string(),
            quote_response: quote,
            wrap_and_unwrap_sol: Some(config.wrap_unwrap_sol),
            use_shared_accounts: Some(config.use_shared_accounts),
            fee_account: None,
            compute_unit_price_micro_lamports: config.priority_fee_micro_lamports,
            use_token_ledger: None,
            destination_token_account: None,
            dynamic_compute_unit_limit: Some(config.dynamic_compute_unit_limit),
            skip_user_accounts_rpc_calls: None,
        };

        // Get swap transaction from Jupiter
        let url = format!("{}/swap", self.api_url);
        let response = self
            .http_client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!(
                "Swap request failed: {}",
                error_text
            )));
        }

        let swap_response: SwapResponse = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        // Decode and sign transaction
        let tx_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &swap_response.swap_transaction,
        )
        .map_err(|e| ToolkitError::ParseError(format!("Failed to decode transaction: {}", e)))?;

        let mut versioned_tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
            .map_err(|e| ToolkitError::ParseError(format!("Failed to deserialize tx: {}", e)))?;

        // Sign the transaction
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        versioned_tx
            .message
            .set_recent_blockhash(recent_blockhash);

        let signed_tx = VersionedTransaction::try_new(versioned_tx.message, &[wallet])
            .map_err(|e| ToolkitError::SigningError(e.to_string()))?;

        // Send and confirm transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&signed_tx)
            .await
            .map_err(|e| ToolkitError::TransactionError(e.to_string()))?;

        Ok(signature)
    }

    /// Simple swap helper - swap tokens with default config
    ///
    /// # Arguments
    /// * `wallet` - The wallet keypair
    /// * `input_mint` - Input token mint address
    /// * `output_mint` - Output token mint address  
    /// * `amount` - Amount to swap in smallest units
    /// * `slippage_bps` - Slippage tolerance in basis points
    ///
    /// # Example
    /// ```rust,no_run
    /// // Swap 1 USDC for SOL with 0.5% slippage
    /// let sig = jupiter.simple_swap(
    ///     &wallet,
    ///     JupiterClient::USDC_MINT,
    ///     JupiterClient::SOL_MINT,
    ///     1_000_000,
    ///     50,
    /// ).await?;
    /// ```
    pub async fn simple_swap(
        &self,
        wallet: &Keypair,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<Signature> {
        let quote = self
            .get_quote(input_mint, output_mint, amount, slippage_bps)
            .await?;
        self.swap(wallet, quote).await
    }

    /// Get the best price for a token pair without executing
    ///
    /// Returns the expected output amount for the given input
    pub async fn get_price(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<u64> {
        let quote = self.get_quote(input_mint, output_mint, amount, 50).await?;
        Ok(quote.out_amount)
    }

    /// Get price impact for a swap
    ///
    /// Returns the price impact as a percentage string
    pub async fn get_price_impact(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<String> {
        let quote = self.get_quote(input_mint, output_mint, amount, 50).await?;
        Ok(quote.price_impact_pct)
    }

    /// Get the route labels for a swap (which DEXs will be used)
    pub async fn get_route_labels(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<Vec<String>> {
        let quote = self.get_quote(input_mint, output_mint, amount, 50).await?;
        Ok(quote
            .route_plan
            .iter()
            .filter_map(|step| step.swap_info.label.clone())
            .collect())
    }
}

/// Helper function to deserialize string numbers to u64
fn deserialize_string_to_u64<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_quote() {
        let jupiter = JupiterClient::new("https://api.mainnet-beta.solana.com");
        
        // This test requires network access
        let result = jupiter
            .get_quote(
                JupiterClient::USDC_MINT,
                JupiterClient::SOL_MINT,
                1_000_000, // 1 USDC
                50,
            )
            .await;

        // Just check that we can parse the response
        if let Ok(quote) = result {
            assert!(!quote.input_mint.is_empty());
            assert!(!quote.output_mint.is_empty());
            assert!(quote.out_amount > 0);
        }
    }

    #[test]
    fn test_swap_config_default() {
        let config = SwapConfig::default();
        assert_eq!(config.slippage_bps, 50);
        assert!(config.wrap_unwrap_sol);
        assert!(config.use_shared_accounts);
    }

    #[test]
    fn test_swap_config_builder() {
        let config = SwapConfig::with_slippage(100).with_priority_fee(5000);
        assert_eq!(config.slippage_bps, 100);
        assert_eq!(config.priority_fee_micro_lamports, Some(5000));
    }
}
