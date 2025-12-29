//! High-performance execution modules for trading agents.
//!
//! The `speed` module provides optimized components for time-critical Solana operations,
//! designed for trading bots and agents that require minimal latency.
//!
//! # Modules
//!
//! - [`fast_rpc`] - Connection pooling and parallel RPC requests
//! - [`tx_builder`] - Blockhash caching and instant transaction building
//! - [`swap`] - Optimized swap execution with DEX routing
//! - [`jupiter`] - Direct Jupiter API integration
//! - [`sniper`] - Time-critical execution for token launches
//! - [`websocket`] - Real-time event subscriptions
//!
//! # Performance Targets
//!
//! - Cached operations: **0ms latency** (blockhash, ATA computation)
//! - Quote to execution: **<500ms**
//! - Transaction confirmation: **Real-time via websockets**
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use solana_pipkit::speed::prelude::*;
//! use solana_sdk::{signature::Keypair, pubkey::Pubkey};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create high-performance RPC client
//!     let rpc = FastRpcClient::new(vec![
//!         "https://api.mainnet-beta.solana.com".to_string(),
//!     ]);
//!
//!     let payer = Keypair::new();
//!
//!     // Create swap executor
//!     let executor = SwapExecutor::new(rpc.clone(), payer);
//!
//!     // Execute fast swap
//!     let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
//!     let sol = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
//!
//!     let config = SwapConfig::fast();
//!     let result = executor.swap_fast(&usdc, &sol, 1_000_000, config).await?;
//!
//!     println!("Swap completed in {}ms", result.execution_time_ms);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Speed Module                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
//! │  │ FastRpcClient│────▶│ SwapExecutor│────▶│  Sniper     │   │
//! │  └─────────────┘     └─────────────┘     └─────────────┘   │
//! │         │                   │                   │           │
//! │         ▼                   ▼                   ▼           │
//! │  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
//! │  │ TxBuilder   │     │ JupiterApi  │     │ Websocket   │   │
//! │  └─────────────┘     └─────────────┘     └─────────────┘   │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod fast_rpc;
pub mod jupiter;
pub mod sniper;
pub mod swap;
pub mod tx_builder;
pub mod websocket;

// Re-export main types
pub use fast_rpc::{
    EndpointHealth, FastRpcClient, LoadBalanceStrategy, PremiumRpcKeys, RpcEndpoint,
};
pub use jupiter::{
    JupiterApiClient, JupiterQuote, PriceData, RoutePlanStep, SwapInfo, SwapRequest,
    SwapResponse, TokenInfo,
};
pub use sniper::{
    PreparedSnipe, Sniper, SniperConfig, SniperError, SniperResult,
};
pub use swap::{
    PriorityLevel, Quote, SwapConfig, SwapError, SwapExecutor, SwapResult,
};
pub use tx_builder::{FastTransactionBuilder, TransactionTemplate};
pub use websocket::{
    EventCallback, PoolWatcher, SolanaWebsocket, SubscriptionId, WebsocketEvent, WsError,
};

/// Prelude for convenient imports.
///
/// ```rust,ignore
/// use solana_pipkit::speed::prelude::*;
/// ```
pub mod prelude {
    // Fast RPC
    pub use super::{
        EndpointHealth, FastRpcClient, LoadBalanceStrategy, PremiumRpcKeys,
    };

    // Transaction building
    pub use super::{FastTransactionBuilder, TransactionTemplate};

    // Swaps
    pub use super::{
        PriorityLevel, Quote, SwapConfig, SwapError, SwapExecutor, SwapResult,
    };

    // Jupiter
    pub use super::{JupiterApiClient, JupiterQuote};

    // Sniping
    pub use super::{PreparedSnipe, Sniper, SniperConfig, SniperResult};

    // Websockets
    pub use super::{PoolWatcher, SolanaWebsocket, WebsocketEvent};
}

/// Common token mints for trading.
pub mod mints {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        /// Native SOL (wrapped).
        pub static ref SOL: Pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        /// USDC.
        pub static ref USDC: Pubkey = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        /// USDT.
        pub static ref USDT: Pubkey = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
        /// BONK.
        pub static ref BONK: Pubkey = Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap();
        /// JUP.
        pub static ref JUP: Pubkey = Pubkey::from_str("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").unwrap();
        /// WIF.
        pub static ref WIF: Pubkey = Pubkey::from_str("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm").unwrap();
    }
}

/// Known DEX program IDs.
pub mod dex {
    pub use super::sniper::dex_programs::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mints() {
        assert_eq!(
            mints::SOL.to_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            mints::USDC.to_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }

    #[test]
    fn test_priority_levels() {
        assert_eq!(PriorityLevel::None.to_microlamports(), 0);
        assert!(PriorityLevel::Turbo.to_microlamports() > PriorityLevel::High.to_microlamports());
    }

    #[test]
    fn test_swap_config_presets() {
        let fast = SwapConfig::fast();
        assert!(fast.direct_route_only);

        let guaranteed = SwapConfig::guaranteed();
        assert!(guaranteed.use_jito);
    }
}
