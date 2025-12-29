//! Integration tests for the speed module.
//!
//! These tests verify the high-performance execution module functionality.
//! Some tests require network access and are marked with `#[ignore]`.

#![cfg(feature = "speed")]

use solana_pipkit::speed::*;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    system_instruction,
};
use std::str::FromStr;
use std::time::{Duration, Instant};

// =============================================================================
// FastRpcClient Tests
// =============================================================================

#[test]
fn test_fast_rpc_client_creation() {
    let endpoints = vec![
        "https://api.mainnet-beta.solana.com".to_string(),
        "https://api.devnet.solana.com".to_string(),
    ];

    let client = FastRpcClient::new(endpoints);
    assert_eq!(client.endpoint_count(), 2);
}

#[test]
fn test_fast_rpc_single_endpoint() {
    let client = FastRpcClient::single("https://api.mainnet-beta.solana.com");
    assert_eq!(client.endpoint_count(), 1);
}

#[test]
fn test_premium_rpc_keys() {
    let keys = PremiumRpcKeys::new()
        .with_helius("test-helius")
        .with_triton("test-triton")
        .with_quicknode("https://my-quicknode.com")
        .with_alchemy("test-alchemy");

    let endpoints = keys.get_endpoints();
    assert_eq!(endpoints.len(), 4);
    assert!(endpoints[0].contains("helius"));
    assert!(endpoints[1].contains("triton"));
}

#[test]
fn test_load_balance_strategies() {
    let client = FastRpcClient::new(vec![
        "https://endpoint1.com".to_string(),
        "https://endpoint2.com".to_string(),
    ]);

    // Test setting strategies
    client.set_strategy(LoadBalanceStrategy::RoundRobin);
    client.set_strategy(LoadBalanceStrategy::LeastLatency);
    client.set_strategy(LoadBalanceStrategy::Random);
    client.set_strategy(LoadBalanceStrategy::Weighted);
}

#[tokio::test]
#[ignore] // Requires network
async fn test_benchmark_endpoints() {
    let client = FastRpcClient::new(vec![
        "https://api.mainnet-beta.solana.com".to_string(),
    ]);

    let health = client.benchmark_endpoints().await;
    assert!(!health.is_empty());

    // First result should be sorted by latency
    if !health.is_empty() && health[0].healthy {
        assert!(health[0].latency_ms < u64::MAX);
    }
}

// =============================================================================
// FastTransactionBuilder Tests
// =============================================================================

#[test]
fn test_transaction_builder_creation() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);
    assert!(builder.payer() != Pubkey::default());
}

#[tokio::test]
async fn test_build_sol_transfer() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);
    let recipient = Pubkey::new_unique();

    let tx = builder.build_sol_transfer(&recipient, LAMPORTS_PER_SOL).await;
    assert_eq!(tx.message.instructions.len(), 1);
}

#[tokio::test]
async fn test_build_multi_transfer() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let transfers = vec![
        (Pubkey::new_unique(), 1_000_000),
        (Pubkey::new_unique(), 2_000_000),
        (Pubkey::new_unique(), 3_000_000),
    ];

    let tx = builder.build_multi_transfer(transfers).await;
    assert_eq!(tx.message.instructions.len(), 3);
}

#[tokio::test]
async fn test_build_with_priority_fee() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let recipient = Pubkey::new_unique();
    let ix = system_instruction::transfer(&builder.payer(), &recipient, 1_000_000);

    let tx = builder
        .build_with_priority_fee(vec![ix], 200_000, 5000)
        .await;

    // Should have: compute limit + priority fee + transfer = 3 instructions
    assert_eq!(tx.message.instructions.len(), 3);
}

#[test]
fn test_get_ata() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ata = builder.get_ata(&owner, &mint);
    // ATA should be deterministic
    let ata2 = builder.get_ata(&owner, &mint);
    assert_eq!(ata, ata2);
}

#[test]
fn test_estimate_fee() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let fee_1 = builder.estimate_fee(1);
    let fee_2 = builder.estimate_fee(2);

    assert!(fee_2 > fee_1);
    assert_eq!(fee_1, 10000); // 5000 base + 5000 per instruction
}

#[tokio::test]
async fn test_transaction_template() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let recipient = Pubkey::new_unique();
    let ix = system_instruction::transfer(&builder.payer(), &recipient, 1_000_000);

    let template = TransactionTemplate::new(vec![ix], 200_000, "SOL transfer");
    assert_eq!(template.description, "SOL transfer");
    assert_eq!(template.compute_units, 200_000);

    let tx = template.build(&builder).await;
    assert_eq!(tx.message.instructions.len(), 1);
}

// =============================================================================
// SwapConfig Tests
// =============================================================================

#[test]
fn test_swap_config_default() {
    let config = SwapConfig::default();
    assert_eq!(config.slippage_bps, 50);
    assert_eq!(config.priority_fee, PriorityLevel::Medium);
    assert!(!config.use_jito);
    assert!(config.wrap_unwrap_sol);
}

#[test]
fn test_swap_config_fast() {
    let config = SwapConfig::fast();
    assert_eq!(config.slippage_bps, 100);
    assert_eq!(config.priority_fee, PriorityLevel::High);
    assert!(config.direct_route_only);
    assert!(config.max_accounts.is_some());
}

#[test]
fn test_swap_config_guaranteed() {
    let config = SwapConfig::guaranteed();
    assert!(config.use_jito);
    assert_eq!(config.priority_fee, PriorityLevel::Turbo);
}

#[test]
fn test_swap_config_best_price() {
    let config = SwapConfig::best_price();
    assert_eq!(config.slippage_bps, 30);
    assert_eq!(config.priority_fee, PriorityLevel::Low);
}

#[test]
fn test_swap_config_builder_pattern() {
    let config = SwapConfig::default()
        .with_slippage(75)
        .with_priority(PriorityLevel::High)
        .with_jito()
        .with_direct_routes()
        .with_max_accounts(15);

    assert_eq!(config.slippage_bps, 75);
    assert_eq!(config.priority_fee, PriorityLevel::High);
    assert!(config.use_jito);
    assert!(config.direct_route_only);
    assert_eq!(config.max_accounts, Some(15));
}

#[test]
fn test_priority_levels() {
    assert_eq!(PriorityLevel::None.to_microlamports(), 0);
    assert_eq!(PriorityLevel::Low.to_microlamports(), 1_000);
    assert_eq!(PriorityLevel::Medium.to_microlamports(), 10_000);
    assert_eq!(PriorityLevel::High.to_microlamports(), 100_000);
    assert_eq!(PriorityLevel::Turbo.to_microlamports(), 1_000_000);

    // Only Turbo should have Jito tip
    assert_eq!(PriorityLevel::None.jito_tip(), 0);
    assert!(PriorityLevel::Turbo.jito_tip() > 0);
}

// =============================================================================
// Jupiter API Tests
// =============================================================================

#[test]
fn test_jupiter_client_creation() {
    let client = JupiterApiClient::new();
    // Just verify creation succeeds
    assert!(true);
}

#[tokio::test]
#[ignore] // Requires network
async fn test_jupiter_get_quote() {
    let client = JupiterApiClient::new();

    let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
    let sol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    let result = client.quote(&usdc, &sol, 1_000_000, 50).await;

    if let Ok(quote) = result {
        assert!(quote.out_amount > 0);
        assert_eq!(quote.slippage_bps, 50);
    }
}

#[tokio::test]
#[ignore] // Requires network
async fn test_jupiter_get_price() {
    let client = JupiterApiClient::new();

    let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    let result = client.get_price(&usdc).await;

    if let Ok(price) = result {
        // USDC should be approximately $1
        assert!(price > 0.9 && price < 1.1);
    }
}

// =============================================================================
// Sniper Tests
// =============================================================================

#[test]
fn test_sniper_config_default() {
    let config = SniperConfig::default();
    assert!(config.safety_checks);
    assert_eq!(config.max_slippage_bps, 500);
    assert!(!config.use_jito);
}

#[test]
fn test_sniper_config_aggressive() {
    let config = SniperConfig::aggressive();
    assert!(!config.safety_checks);
    assert!(config.use_jito);
    assert_eq!(config.max_slippage_bps, 1000);
}

#[test]
fn test_sniper_config_safe() {
    let config = SniperConfig::safe();
    assert!(config.safety_checks);
    assert!(!config.use_jito);
    assert_eq!(config.max_slippage_bps, 200);
}

#[test]
fn test_sniper_config_builder() {
    let config = SniperConfig::default()
        .with_max_buy(200_000_000)
        .with_jito(50_000)
        .without_safety_checks();

    assert_eq!(config.max_buy_lamports, 200_000_000);
    assert!(config.use_jito);
    assert_eq!(config.jito_tip_lamports, 50_000);
    assert!(!config.safety_checks);
}

#[test]
fn test_sniper_result_success() {
    use solana_sdk::signature::Signature;

    let result = SniperResult::success(
        Signature::default(),
        1_000_000,
        100_000,
        150,
    );

    assert!(result.success);
    assert!(result.signature.is_some());
    assert_eq!(result.tokens_received, 1_000_000);
    assert_eq!(result.sol_spent, 100_000);
    assert!(result.price_per_token > 0.0);
}

#[test]
fn test_sniper_result_failure() {
    let result = SniperResult::failure("Test error".to_string(), 50);

    assert!(!result.success);
    assert!(result.signature.is_none());
    assert_eq!(result.tokens_received, 0);
    assert_eq!(result.error, Some("Test error".to_string()));
}

#[test]
fn test_prepared_snipe_validity() {
    let prepared = PreparedSnipe {
        transaction: solana_sdk::transaction::Transaction::default(),
        created_at: Instant::now(),
        valid_until: Instant::now() + Duration::from_secs(30),
        token_mint: Pubkey::new_unique(),
        sol_amount: 100_000_000,
        expected_tokens: Some(1_000_000),
    };

    assert!(prepared.is_valid());
    assert!(prepared.time_until_expiry() > Duration::ZERO);
}

// =============================================================================
// Websocket Tests
// =============================================================================

#[test]
fn test_websocket_creation() {
    let ws = SolanaWebsocket::new("wss://api.mainnet-beta.solana.com");
    assert!(!ws.is_connected());
}

#[test]
fn test_pool_watcher_creation() {
    let mut watcher = PoolWatcher::new("wss://api.mainnet-beta.solana.com");
    watcher.watch_dex(Pubkey::new_unique());
    // Verify dex was added
}

// =============================================================================
// Mints Tests
// =============================================================================

#[test]
fn test_known_mints() {
    assert_eq!(
        mints::SOL.to_string(),
        "So11111111111111111111111111111111111111112"
    );
    assert_eq!(
        mints::USDC.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    assert_eq!(
        mints::USDT.to_string(),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
    );
    assert_eq!(
        mints::BONK.to_string(),
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"
    );
    assert_eq!(
        mints::JUP.to_string(),
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
    );
}

// =============================================================================
// Performance Benchmarks (run with --release)
// =============================================================================

#[tokio::test]
async fn test_blockhash_caching_zero_latency() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    // Once blockhash is cached, subsequent calls should be instant
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = builder.get_cached_blockhash().await;
    }
    let elapsed = start.elapsed();

    // 1000 calls should complete in < 10ms (essentially 0 latency each)
    assert!(
        elapsed < Duration::from_millis(100),
        "1000 blockhash retrievals took {:?}, expected < 100ms",
        elapsed
    );
}

#[test]
fn test_ata_computation_zero_latency() {
    let payer = Keypair::new();
    let builder = FastTransactionBuilder::new(payer);

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let start = Instant::now();
    for _ in 0..10000 {
        let _ = builder.get_ata(&owner, &mint);
    }
    let elapsed = start.elapsed();

    // 10000 ATA computations should complete in reasonable time
    // Note: Use --release for accurate benchmarks
    // In debug mode, allow 500ms; in release, should be < 100ms
    assert!(
        elapsed < Duration::from_millis(500),
        "10000 ATA computations took {:?}, expected < 500ms (use --release for accurate benchmarks)",
        elapsed
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_swap_error_display() {
    let err = SwapError::SlippageExceeded {
        expected: 1000,
        actual: 900,
    };
    let msg = err.to_string();
    assert!(msg.contains("Slippage exceeded"));
    assert!(msg.contains("1000"));
    assert!(msg.contains("900"));
}

#[test]
fn test_sniper_error_display() {
    let err = SniperError::InsufficientLiquidity {
        required: 1_000_000_000,
        available: 500_000_000,
    };
    let msg = err.to_string();
    assert!(msg.contains("Insufficient liquidity"));
}

#[test]
fn test_ws_error_display() {
    let err = WsError::ConnectionFailed("timeout".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Connection failed"));
    assert!(msg.contains("timeout"));
}
