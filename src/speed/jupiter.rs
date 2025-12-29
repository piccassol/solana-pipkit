//! Direct Jupiter API integration for fastest quotes and swaps.
//!
//! This module provides a high-performance Jupiter client optimized for trading agents
//! that require the fastest possible quote-to-execution times.
//!
//! # Features
//! - Direct API access without intermediate caching
//! - Parallel quote fetching
//! - Token list caching
//! - Price feeds
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::speed::JupiterApiClient;
//! use solana_sdk::pubkey::Pubkey;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let jupiter = JupiterApiClient::new();
//!
//!     let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
//!     let sol = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
//!
//!     // Get quote
//!     let quote = jupiter.quote(&usdc, &sol, 1_000_000, 50).await?;
//!     println!("Expected output: {} lamports", quote.out_amount);
//!
//!     Ok(())
//! }
//! ```

use crate::{Result, ToolkitError};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Jupiter API base URL.
pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";

/// Jupiter price API URL.
pub const JUPITER_PRICE_API: &str = "https://price.jup.ag/v6";

/// Jupiter token list API.
pub const JUPITER_TOKEN_API: &str = "https://token.jup.ag/all";

/// Jupiter API client for direct integration.
pub struct JupiterApiClient {
    /// HTTP client.
    http: reqwest::Client,
    /// API URL.
    api_url: String,
    /// Cached token list.
    token_cache: RwLock<Option<TokenCache>>,
}

/// Cached token list.
struct TokenCache {
    tokens: Vec<TokenInfo>,
    by_address: HashMap<String, TokenInfo>,
    fetched_at: Instant,
}

/// Jupiter quote response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuote {
    /// Input mint.
    pub input_mint: String,
    /// Output mint.
    pub output_mint: String,
    /// Input amount.
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub in_amount: u64,
    /// Output amount.
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub out_amount: u64,
    /// Other amount threshold (for slippage).
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub other_amount_threshold: u64,
    /// Swap mode.
    pub swap_mode: String,
    /// Slippage in basis points.
    pub slippage_bps: u16,
    /// Price impact percentage.
    pub price_impact_pct: String,
    /// Route plan.
    pub route_plan: Vec<RoutePlanStep>,
    /// Context slot.
    #[serde(default)]
    pub context_slot: Option<u64>,
    /// Time taken.
    #[serde(default)]
    pub time_taken: Option<f64>,
}

impl JupiterQuote {
    /// Get input mint as Pubkey.
    pub fn input_mint_pubkey(&self) -> Result<Pubkey> {
        self.input_mint
            .parse()
            .map_err(|_| ToolkitError::ParseError("Invalid input mint".to_string()))
    }

    /// Get output mint as Pubkey.
    pub fn output_mint_pubkey(&self) -> Result<Pubkey> {
        self.output_mint
            .parse()
            .map_err(|_| ToolkitError::ParseError("Invalid output mint".to_string()))
    }

    /// Get price impact as f64.
    pub fn price_impact(&self) -> f64 {
        self.price_impact_pct.parse().unwrap_or(0.0)
    }

    /// Get route labels (DEX names).
    pub fn route_labels(&self) -> Vec<String> {
        self.route_plan
            .iter()
            .filter_map(|step| step.swap_info.label.clone())
            .collect()
    }
}

/// Route plan step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    /// Swap info.
    pub swap_info: SwapInfo,
    /// Percentage of input.
    pub percent: u8,
}

/// Swap information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    /// AMM key.
    pub amm_key: String,
    /// Label (DEX name).
    pub label: Option<String>,
    /// Input mint.
    pub input_mint: String,
    /// Output mint.
    pub output_mint: String,
    /// Input amount.
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub in_amount: u64,
    /// Output amount.
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub out_amount: u64,
    /// Fee amount.
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub fee_amount: u64,
    /// Fee mint.
    pub fee_mint: String,
}

impl SwapInfo {
    /// Get AMM key as Pubkey.
    pub fn amm_pubkey(&self) -> Result<Pubkey> {
        self.amm_key
            .parse()
            .map_err(|_| ToolkitError::ParseError("Invalid AMM key".to_string()))
    }
}

/// Swap request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    /// User public key.
    pub user_public_key: String,
    /// Quote response.
    pub quote_response: JupiterQuote,
    /// Wrap and unwrap SOL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_and_unwrap_sol: Option<bool>,
    /// Use shared accounts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_shared_accounts: Option<bool>,
    /// Priority fee in micro-lamports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    /// Dynamic compute unit limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_compute_unit_limit: Option<bool>,
    /// As legacy transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_legacy_transaction: Option<bool>,
}

/// Swap response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    /// Base64-encoded transaction.
    pub swap_transaction: String,
    /// Last valid block height.
    pub last_valid_block_height: u64,
    /// Priority fee type.
    #[serde(default)]
    pub priority_fee: Option<String>,
}

/// Token information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token address.
    pub address: String,
    /// Token symbol.
    pub symbol: String,
    /// Token name.
    pub name: String,
    /// Decimals.
    pub decimals: u8,
    /// Logo URI.
    #[serde(default)]
    pub logo_uri: Option<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Daily volume.
    #[serde(default)]
    pub daily_volume: Option<f64>,
}

/// Price response.
#[derive(Debug, Clone, Deserialize)]
pub struct PriceResponse {
    /// Price data by mint.
    pub data: HashMap<String, PriceData>,
}

/// Price data for a token.
#[derive(Debug, Clone, Deserialize)]
pub struct PriceData {
    /// Token ID/address.
    pub id: String,
    /// Mint type.
    #[serde(rename = "type")]
    pub mint_type: String,
    /// Price in USD.
    pub price: f64,
}

impl JupiterApiClient {
    /// Create a new Jupiter client.
    pub fn new() -> Self {
        Self::with_url(JUPITER_API_URL)
    }

    /// Create with custom API URL.
    pub fn with_url(api_url: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_url: api_url.to_string(),
            token_cache: RwLock::new(None),
        }
    }

    /// Get quote from Jupiter.
    pub async fn quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!("Quote failed: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))
    }

    /// Get quote with additional options.
    pub async fn quote_with_options(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16,
        only_direct_routes: bool,
        max_accounts: Option<u8>,
    ) -> Result<JupiterQuote> {
        let mut url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        if only_direct_routes {
            url.push_str("&onlyDirectRoutes=true");
        }

        if let Some(max) = max_accounts {
            url.push_str(&format!("&maxAccounts={}", max));
        }

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!("Quote failed: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))
    }

    /// Get swap transaction from Jupiter.
    pub async fn swap_transaction(
        &self,
        quote: &JupiterQuote,
        user_pubkey: &Pubkey,
    ) -> Result<VersionedTransaction> {
        self.swap_transaction_with_options(quote, user_pubkey, None, true, false).await
    }

    /// Get swap transaction with options.
    pub async fn swap_transaction_with_options(
        &self,
        quote: &JupiterQuote,
        user_pubkey: &Pubkey,
        priority_fee_microlamports: Option<u64>,
        wrap_unwrap_sol: bool,
        as_legacy: bool,
    ) -> Result<VersionedTransaction> {
        let request = SwapRequest {
            user_public_key: user_pubkey.to_string(),
            quote_response: quote.clone(),
            wrap_and_unwrap_sol: Some(wrap_unwrap_sol),
            use_shared_accounts: Some(true),
            compute_unit_price_micro_lamports: priority_fee_microlamports,
            dynamic_compute_unit_limit: Some(true),
            as_legacy_transaction: Some(as_legacy),
        };

        let url = format!("{}/swap", self.api_url);
        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ToolkitError::JupiterError(format!("Swap failed: {}", error)));
        }

        let swap_response: SwapResponse = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        // Decode transaction
        let tx_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &swap_response.swap_transaction,
        )
        .map_err(|e| ToolkitError::ParseError(format!("Failed to decode transaction: {}", e)))?;

        bincode::deserialize(&tx_bytes)
            .map_err(|e| ToolkitError::ParseError(format!("Failed to deserialize transaction: {}", e)))
    }

    /// Get all available tokens.
    pub async fn get_tokens(&self) -> Result<Vec<TokenInfo>> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(ref c) = *cache {
                // Cache valid for 1 hour
                if c.fetched_at.elapsed() < Duration::from_secs(3600) {
                    return Ok(c.tokens.clone());
                }
            }
        }

        let response = self
            .http
            .get(JUPITER_TOKEN_API)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ToolkitError::JupiterError("Failed to fetch tokens".to_string()));
        }

        let tokens: Vec<TokenInfo> = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        // Update cache
        let by_address: HashMap<String, TokenInfo> = tokens
            .iter()
            .map(|t| (t.address.clone(), t.clone()))
            .collect();

        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                tokens: tokens.clone(),
                by_address,
                fetched_at: Instant::now(),
            });
        }

        Ok(tokens)
    }

    /// Get token info by address.
    pub async fn get_token(&self, address: &Pubkey) -> Result<TokenInfo> {
        let tokens = self.get_tokens().await?;
        let addr = address.to_string();

        tokens
            .into_iter()
            .find(|t| t.address == addr)
            .ok_or_else(|| ToolkitError::Custom(format!("Token not found: {}", address)))
    }

    /// Get price for token in USD.
    pub async fn get_price(&self, mint: &Pubkey) -> Result<f64> {
        let prices = self.get_prices(&[*mint]).await?;
        prices
            .get(&mint.to_string())
            .map(|p| p.price)
            .ok_or_else(|| ToolkitError::Custom(format!("Price not found for {}", mint)))
    }

    /// Get prices for multiple tokens.
    pub async fn get_prices(&self, mints: &[Pubkey]) -> Result<HashMap<String, PriceData>> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let ids: Vec<String> = mints.iter().map(|m| m.to_string()).collect();
        let url = format!("{}/price?ids={}", JUPITER_PRICE_API, ids.join(","));

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ToolkitError::JupiterError("Failed to fetch prices".to_string()));
        }

        let price_response: PriceResponse = response
            .json()
            .await
            .map_err(|e| ToolkitError::ParseError(e.to_string()))?;

        Ok(price_response.data)
    }

    /// Get multiple quotes in parallel.
    pub async fn get_quotes_parallel(
        &self,
        requests: Vec<(Pubkey, Pubkey, u64, u16)>,
    ) -> Vec<Result<JupiterQuote>> {
        let handles: Vec<_> = requests
            .into_iter()
            .map(|(input, output, amount, slippage)| {
                let client = self.http.clone();
                let url = format!(
                    "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
                    self.api_url, input, output, amount, slippage
                );

                tokio::spawn(async move {
                    let response = client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|e| ToolkitError::NetworkError(e.to_string()))?;

                    if !response.status().is_success() {
                        let error = response.text().await.unwrap_or_default();
                        return Err(ToolkitError::JupiterError(format!("Quote failed: {}", error)));
                    }

                    response
                        .json::<JupiterQuote>()
                        .await
                        .map_err(|e| ToolkitError::ParseError(e.to_string()))
                })
            })
            .collect();

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ToolkitError::Custom(e.to_string()))),
            }
        }

        results
    }
}

impl Default for JupiterApiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to deserialize string numbers to u64.
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
    use std::str::FromStr;

    #[test]
    fn test_jupiter_client_creation() {
        let client = JupiterApiClient::new();
        assert_eq!(client.api_url, JUPITER_API_URL);
    }

    #[test]
    fn test_jupiter_quote_methods() {
        let quote = JupiterQuote {
            input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            output_mint: "So11111111111111111111111111111111111111112".to_string(),
            in_amount: 1_000_000,
            out_amount: 5_000_000,
            other_amount_threshold: 4_900_000,
            swap_mode: "ExactIn".to_string(),
            slippage_bps: 50,
            price_impact_pct: "0.5".to_string(),
            route_plan: vec![],
            context_slot: Some(12345),
            time_taken: Some(100.0),
        };

        assert_eq!(quote.price_impact(), 0.5);
        assert!(quote.input_mint_pubkey().is_ok());
        assert!(quote.output_mint_pubkey().is_ok());
    }
}
