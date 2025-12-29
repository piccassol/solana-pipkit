//! High-performance RPC client with connection pooling and parallel requests.
//!
//! This module provides an optimized RPC client designed for trading agents
//! that require minimal latency and maximum reliability.
//!
//! # Features
//! - Connection pooling across multiple endpoints
//! - Automatic failover and retry logic
//! - Latency-based load balancing
//! - Parallel account fetching
//! - Support for premium RPC providers (Helius, Triton, QuickNode)
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::speed::{FastRpcClient, LoadBalanceStrategy};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let endpoints = vec![
//!         "https://api.mainnet-beta.solana.com".to_string(),
//!         "https://solana-api.projectserum.com".to_string(),
//!     ];
//!
//!     let mut client = FastRpcClient::new(endpoints);
//!     client.set_strategy(LoadBalanceStrategy::LeastLatency);
//!
//!     // Benchmark and sort by latency
//!     let health = client.benchmark_endpoints().await;
//!     println!("Fastest endpoint: {} ({}ms)", health[0].url, health[0].latency_ms);
//!
//!     // Fast slot fetch
//!     let slot = client.get_slot_fast().await?;
//!     println!("Current slot: {}", slot);
//!
//!     Ok(())
//! }
//! ```

use crate::{Result, ToolkitError};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    clock::Slot,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Load balancing strategy for RPC endpoint selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadBalanceStrategy {
    /// Round-robin across all endpoints.
    #[default]
    RoundRobin,
    /// Select endpoint with lowest average latency.
    LeastLatency,
    /// Random endpoint selection.
    Random,
    /// Weighted random based on endpoint weights.
    Weighted,
}

/// RPC endpoint with performance tracking.
pub struct RpcEndpoint {
    /// Endpoint URL.
    pub url: String,
    /// Weight for weighted load balancing (1-100).
    pub weight: u8,
    /// Average latency in milliseconds (rolling average).
    pub avg_latency_ms: AtomicU64,
    /// Request count for this endpoint.
    pub request_count: AtomicU64,
    /// Error count for this endpoint.
    pub error_count: AtomicU64,
    /// Whether endpoint is currently healthy.
    pub healthy: std::sync::atomic::AtomicBool,
    /// RPC client instance.
    client: RpcClient,
}

impl std::fmt::Debug for RpcEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcEndpoint")
            .field("url", &self.url)
            .field("weight", &self.weight)
            .field("avg_latency_ms", &self.avg_latency_ms.load(Ordering::Relaxed))
            .field("request_count", &self.request_count.load(Ordering::Relaxed))
            .field("error_count", &self.error_count.load(Ordering::Relaxed))
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl RpcEndpoint {
    /// Create a new endpoint with default weight.
    pub fn new(url: &str) -> Self {
        Self::with_weight(url, 50)
    }

    /// Create a new endpoint with custom weight.
    pub fn with_weight(url: &str, weight: u8) -> Self {
        Self {
            url: url.to_string(),
            weight: weight.clamp(1, 100),
            avg_latency_ms: AtomicU64::new(0),
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            healthy: std::sync::atomic::AtomicBool::new(true),
            client: RpcClient::new_with_commitment(
                url.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    /// Update latency with exponential moving average.
    fn update_latency(&self, latency_ms: u64) {
        let current = self.avg_latency_ms.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            latency_ms
        } else {
            // EMA with alpha = 0.3
            (current * 7 + latency_ms * 3) / 10
        };
        self.avg_latency_ms.store(new_avg, Ordering::Relaxed);
    }

    /// Record a successful request.
    fn record_success(&self, latency_ms: u64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.update_latency(latency_ms);
        self.healthy.store(true, Ordering::Relaxed);
    }

    /// Record a failed request.
    fn record_error(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.error_count.fetch_add(1, Ordering::Relaxed);

        // Mark unhealthy if error rate exceeds 50%
        let total = self.request_count.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        if total > 10 && errors * 2 > total {
            self.healthy.store(false, Ordering::Relaxed);
        }
    }
}

/// Premium RPC provider API keys.
#[derive(Debug, Clone, Default)]
pub struct PremiumRpcKeys {
    /// Helius API key.
    pub helius: Option<String>,
    /// Triton API key.
    pub triton: Option<String>,
    /// QuickNode API key.
    pub quicknode: Option<String>,
    /// Alchemy API key.
    pub alchemy: Option<String>,
}

impl PremiumRpcKeys {
    /// Create new empty keys.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Helius API key.
    pub fn with_helius(mut self, key: impl Into<String>) -> Self {
        self.helius = Some(key.into());
        self
    }

    /// Set Triton API key.
    pub fn with_triton(mut self, key: impl Into<String>) -> Self {
        self.triton = Some(key.into());
        self
    }

    /// Set QuickNode API key.
    pub fn with_quicknode(mut self, key: impl Into<String>) -> Self {
        self.quicknode = Some(key.into());
        self
    }

    /// Set Alchemy API key.
    pub fn with_alchemy(mut self, key: impl Into<String>) -> Self {
        self.alchemy = Some(key.into());
        self
    }

    /// Get all configured endpoint URLs.
    pub fn get_endpoints(&self) -> Vec<String> {
        let mut endpoints = Vec::new();

        if let Some(key) = &self.helius {
            endpoints.push(format!("https://mainnet.helius-rpc.com/?api-key={}", key));
        }

        if let Some(key) = &self.triton {
            endpoints.push(format!("https://rpc.triton.one/?api-key={}", key));
        }

        if let Some(key) = &self.quicknode {
            // QuickNode URLs are typically full URLs with the key embedded
            endpoints.push(key.clone());
        }

        if let Some(key) = &self.alchemy {
            endpoints.push(format!("https://solana-mainnet.g.alchemy.com/v2/{}", key));
        }

        endpoints
    }
}

/// Endpoint health status.
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    /// Endpoint URL.
    pub url: String,
    /// Measured latency in milliseconds.
    pub latency_ms: u64,
    /// Whether endpoint responded successfully.
    pub healthy: bool,
    /// Current slot from endpoint.
    pub slot: u64,
    /// Error message if unhealthy.
    pub error: Option<String>,
}

/// High-performance RPC client with connection pooling.
pub struct FastRpcClient {
    /// RPC endpoints pool.
    endpoints: Vec<Arc<RpcEndpoint>>,
    /// Current endpoint index for round-robin.
    current: AtomicUsize,
    /// Load balancing strategy.
    strategy: RwLock<LoadBalanceStrategy>,
    /// Maximum retry attempts.
    max_retries: usize,
    /// Timeout for single requests.
    timeout: Duration,
}

impl FastRpcClient {
    /// Create with multiple RPC endpoints for redundancy and speed.
    pub fn new(endpoint_urls: Vec<String>) -> Self {
        let endpoints = endpoint_urls
            .into_iter()
            .map(|url| Arc::new(RpcEndpoint::new(&url)))
            .collect();

        Self {
            endpoints,
            current: AtomicUsize::new(0),
            strategy: RwLock::new(LoadBalanceStrategy::RoundRobin),
            max_retries: 3,
            timeout: Duration::from_secs(10),
        }
    }

    /// Create with premium RPC endpoints.
    pub fn with_premium_endpoints(api_keys: PremiumRpcKeys) -> Self {
        let mut endpoints = api_keys.get_endpoints();

        // Add public fallbacks
        endpoints.push("https://api.mainnet-beta.solana.com".to_string());

        Self::new(endpoints)
    }

    /// Create with single endpoint.
    pub fn single(url: &str) -> Self {
        Self::new(vec![url.to_string()])
    }

    /// Set load balancing strategy.
    pub fn set_strategy(&self, strategy: LoadBalanceStrategy) {
        let mut guard = self.strategy.blocking_write();
        *guard = strategy;
    }

    /// Set maximum retry attempts.
    pub fn set_max_retries(&mut self, retries: usize) {
        self.max_retries = retries;
    }

    /// Set request timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get number of configured endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Select next endpoint based on strategy.
    async fn select_endpoint(&self) -> Arc<RpcEndpoint> {
        let strategy = *self.strategy.read().await;

        // Filter healthy endpoints
        let healthy: Vec<_> = self.endpoints
            .iter()
            .filter(|e| e.healthy.load(Ordering::Relaxed))
            .cloned()
            .collect();

        let candidates = if healthy.is_empty() {
            // Fall back to all endpoints if none healthy
            &self.endpoints
        } else {
            &healthy
        };

        match strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.current.fetch_add(1, Ordering::Relaxed) % candidates.len();
                candidates[idx].clone()
            }
            LoadBalanceStrategy::LeastLatency => {
                candidates
                    .iter()
                    .min_by_key(|e| {
                        let latency = e.avg_latency_ms.load(Ordering::Relaxed);
                        if latency == 0 { u64::MAX / 2 } else { latency }
                    })
                    .cloned()
                    .unwrap_or_else(|| candidates[0].clone())
            }
            LoadBalanceStrategy::Random => {
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hasher};
                let idx = RandomState::new().build_hasher().finish() as usize % candidates.len();
                candidates[idx].clone()
            }
            LoadBalanceStrategy::Weighted => {
                // Simple weighted selection
                let total_weight: u64 = candidates.iter().map(|e| e.weight as u64).sum();
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hasher};
                let mut target = (RandomState::new().build_hasher().finish() % total_weight) as u64;

                for endpoint in candidates {
                    if target < endpoint.weight as u64 {
                        return endpoint.clone();
                    }
                    target -= endpoint.weight as u64;
                }
                candidates[0].clone()
            }
        }
    }

    /// Execute request with retry and failover.
    async fn execute_with_retry<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn(Arc<RpcEndpoint>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = ToolkitError::NetworkError("No endpoints available".to_string());
        let mut tried_endpoints = Vec::new();

        for _ in 0..self.max_retries {
            let endpoint = self.select_endpoint().await;

            // Skip if we already tried this endpoint
            if tried_endpoints.contains(&endpoint.url) && tried_endpoints.len() < self.endpoints.len() {
                continue;
            }
            tried_endpoints.push(endpoint.url.clone());

            let start = Instant::now();

            match tokio::time::timeout(self.timeout, operation(endpoint.clone())).await {
                Ok(Ok(result)) => {
                    endpoint.record_success(start.elapsed().as_millis() as u64);
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    endpoint.record_error();
                    last_error = e;
                }
                Err(_) => {
                    endpoint.record_error();
                    last_error = ToolkitError::Timeout(format!(
                        "Request to {} timed out",
                        endpoint.url
                    ));
                }
            }
        }

        Err(last_error)
    }

    /// Send transaction with automatic retry and endpoint failover.
    pub async fn send_transaction_fast(&self, transaction: &Transaction) -> Result<Signature> {
        let tx = transaction.clone();
        self.execute_with_retry(|endpoint| {
            let tx = tx.clone();
            async move {
                endpoint
                    .client
                    .send_and_confirm_transaction(&tx)
                    .await
                    .map_err(|e| ToolkitError::TransactionError(e.to_string()))
            }
        })
        .await
    }

    /// Send transaction without confirmation (faster).
    pub async fn send_transaction_no_confirm(&self, transaction: &Transaction) -> Result<Signature> {
        let tx = transaction.clone();
        self.execute_with_retry(|endpoint| {
            let tx = tx.clone();
            async move {
                endpoint
                    .client
                    .send_transaction(&tx)
                    .await
                    .map_err(|e| ToolkitError::TransactionError(e.to_string()))
            }
        })
        .await
    }

    /// Get multiple accounts in parallel batches.
    pub async fn get_multiple_accounts_fast(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>> {
        if pubkeys.is_empty() {
            return Ok(vec![]);
        }

        // Solana limits to 100 accounts per request
        const BATCH_SIZE: usize = 100;

        let batches: Vec<_> = pubkeys.chunks(BATCH_SIZE).collect();
        let mut handles = Vec::with_capacity(batches.len());

        for batch in batches {
            let batch = batch.to_vec();
            let endpoint = self.select_endpoint().await;

            handles.push(tokio::spawn(async move {
                let start = Instant::now();
                let result = endpoint.client.get_multiple_accounts(&batch).await;

                match &result {
                    Ok(_) => endpoint.record_success(start.elapsed().as_millis() as u64),
                    Err(_) => endpoint.record_error(),
                }

                result.map_err(|e| ToolkitError::RpcError(e))
            }));
        }

        let mut all_accounts = Vec::with_capacity(pubkeys.len());

        for handle in handles {
            let accounts = handle
                .await
                .map_err(|e| ToolkitError::Custom(e.to_string()))??;
            all_accounts.extend(accounts);
        }

        Ok(all_accounts)
    }

    /// Get slot with lowest latency endpoint.
    pub async fn get_slot_fast(&self) -> Result<Slot> {
        self.execute_with_retry(|endpoint| async move {
            endpoint
                .client
                .get_slot()
                .await
                .map_err(|e| ToolkitError::RpcError(e))
        })
        .await
    }

    /// Get latest blockhash with retry.
    pub async fn get_latest_blockhash_fast(&self) -> Result<solana_sdk::hash::Hash> {
        self.execute_with_retry(|endpoint| async move {
            endpoint
                .client
                .get_latest_blockhash()
                .await
                .map_err(|e| ToolkitError::RpcError(e))
        })
        .await
    }

    /// Get account info with retry.
    pub async fn get_account_fast(&self, pubkey: &Pubkey) -> Result<Account> {
        let pk = *pubkey;
        self.execute_with_retry(|endpoint| async move {
            endpoint
                .client
                .get_account(&pk)
                .await
                .map_err(|e| ToolkitError::RpcError(e))
        })
        .await
    }

    /// Get balance with retry.
    pub async fn get_balance_fast(&self, pubkey: &Pubkey) -> Result<u64> {
        let pk = *pubkey;
        self.execute_with_retry(|endpoint| async move {
            endpoint
                .client
                .get_balance(&pk)
                .await
                .map_err(|e| ToolkitError::RpcError(e))
        })
        .await
    }

    /// Health check all endpoints, sort by latency.
    pub async fn benchmark_endpoints(&self) -> Vec<EndpointHealth> {
        let mut handles = Vec::with_capacity(self.endpoints.len());

        for endpoint in &self.endpoints {
            let endpoint = endpoint.clone();
            handles.push(tokio::spawn(async move {
                let start = Instant::now();

                match tokio::time::timeout(
                    Duration::from_secs(5),
                    endpoint.client.get_slot(),
                ).await {
                    Ok(Ok(slot)) => {
                        let latency = start.elapsed().as_millis() as u64;
                        endpoint.record_success(latency);
                        EndpointHealth {
                            url: endpoint.url.clone(),
                            latency_ms: latency,
                            healthy: true,
                            slot,
                            error: None,
                        }
                    }
                    Ok(Err(e)) => {
                        endpoint.record_error();
                        EndpointHealth {
                            url: endpoint.url.clone(),
                            latency_ms: u64::MAX,
                            healthy: false,
                            slot: 0,
                            error: Some(e.to_string()),
                        }
                    }
                    Err(_) => {
                        endpoint.record_error();
                        EndpointHealth {
                            url: endpoint.url.clone(),
                            latency_ms: u64::MAX,
                            healthy: false,
                            slot: 0,
                            error: Some("Timeout".to_string()),
                        }
                    }
                }
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(health) = handle.await {
                results.push(health);
            }
        }

        // Sort by latency (healthy first, then by latency)
        results.sort_by(|a, b| {
            match (a.healthy, b.healthy) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.latency_ms.cmp(&b.latency_ms),
            }
        });

        results
    }

    /// Get the underlying RPC client for the best endpoint.
    pub async fn get_best_client(&self) -> &RpcClient {
        // Find endpoint with lowest latency
        let best = self.endpoints
            .iter()
            .filter(|e| e.healthy.load(Ordering::Relaxed))
            .min_by_key(|e| e.avg_latency_ms.load(Ordering::Relaxed))
            .unwrap_or(&self.endpoints[0]);

        &best.client
    }

    /// Reset all endpoint statistics.
    pub fn reset_stats(&self) {
        for endpoint in &self.endpoints {
            endpoint.request_count.store(0, Ordering::Relaxed);
            endpoint.error_count.store(0, Ordering::Relaxed);
            endpoint.avg_latency_ms.store(0, Ordering::Relaxed);
            endpoint.healthy.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_premium_rpc_keys() {
        let keys = PremiumRpcKeys::new()
            .with_helius("test-helius-key")
            .with_triton("test-triton-key");

        let endpoints = keys.get_endpoints();
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints[0].contains("helius"));
        assert!(endpoints[1].contains("triton"));
    }

    #[test]
    fn test_endpoint_latency_update() {
        let endpoint = RpcEndpoint::new("https://test.com");

        // First update sets the value directly
        endpoint.update_latency(100);
        assert_eq!(endpoint.avg_latency_ms.load(Ordering::Relaxed), 100);

        // Subsequent updates use EMA
        endpoint.update_latency(200);
        let avg = endpoint.avg_latency_ms.load(Ordering::Relaxed);
        assert!(avg > 100 && avg < 200);
    }

    #[test]
    fn test_fast_rpc_client_creation() {
        let client = FastRpcClient::new(vec![
            "https://endpoint1.com".to_string(),
            "https://endpoint2.com".to_string(),
        ]);

        assert_eq!(client.endpoint_count(), 2);
    }

    #[test]
    fn test_single_endpoint() {
        let client = FastRpcClient::single("https://api.mainnet-beta.solana.com");
        assert_eq!(client.endpoint_count(), 1);
    }

    #[tokio::test]
    async fn test_strategy_setting() {
        let client = FastRpcClient::single("https://test.com");

        // Set strategy using the blocking method
        {
            let mut guard = client.strategy.write().await;
            *guard = LoadBalanceStrategy::LeastLatency;
        }

        let strategy = *client.strategy.read().await;
        assert_eq!(strategy, LoadBalanceStrategy::LeastLatency);
    }
}
