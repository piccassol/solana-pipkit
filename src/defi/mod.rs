//! DeFi utilities for interacting with Solana DEXes and liquidity pools.
//!
//! Supports Raydium, Orca, and other major protocols for:
//! - Pool interactions and swaps
//! - Liquidity provision
//! - Yield farming position management
//! - LP token parsing and analysis

mod pools;
mod liquidity;
mod farming;
mod lp_tokens;

pub use pools::*;
pub use liquidity::*;
pub use farming::*;
pub use lp_tokens::*;

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::Result;

/// Known DEX program IDs
pub mod programs {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        pub static ref RAYDIUM_AMM_V4: Pubkey = 
            Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        pub static ref RAYDIUM_CLMM: Pubkey = 
            Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap();
        pub static ref ORCA_WHIRLPOOL: Pubkey = 
            Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        pub static ref METEORA_DLMM: Pubkey = 
            Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap();
    }
}

/// Main DeFi interface.
pub struct DeFi {
    client: RpcClient,
    config: DeFiConfig,
}

#[derive(Debug, Clone)]
pub struct DeFiConfig {
    /// Default slippage tolerance in basis points
    pub default_slippage_bps: u16,
    /// Whether to use Jito for MEV protection
    pub use_jito: bool,
    /// Priority fee in microlamports
    pub priority_fee: u64,
}

impl Default for DeFiConfig {
    fn default() -> Self {
        Self {
            default_slippage_bps: 50, // 0.5%
            use_jito: false,
            priority_fee: 10000,
        }
    }
}

impl DeFi {
    pub fn new(client: RpcClient) -> Self {
        Self {
            client,
            config: DeFiConfig::default(),
        }
    }

    pub fn with_config(client: RpcClient, config: DeFiConfig) -> Self {
        Self { client, config }
    }

    /// Get information about a liquidity pool.
    pub async fn get_pool_info(&self, pool: &Pubkey) -> Result<PoolInfo> {
        let pool_manager = PoolManager::new(&self.client);
        pool_manager.get_info(pool).await
    }

    /// Find best pools for a token pair.
    pub async fn find_pools(&self, token_a: &Pubkey, token_b: &Pubkey) -> Result<Vec<PoolInfo>> {
        let pool_manager = PoolManager::new(&self.client);
        pool_manager.find_pools(token_a, token_b).await
    }

    /// Calculate optimal liquidity provision.
    pub async fn calculate_liquidity(
        &self,
        pool: &Pubkey,
        amount_a: u64,
    ) -> Result<LiquidityQuote> {
        let provider = LiquidityProvider::new(&self.client, self.config.clone());
        provider.quote(pool, amount_a).await
    }

    /// Get farming position details.
    pub async fn get_farm_position(
        &self,
        wallet: &Pubkey,
        farm: &Pubkey,
    ) -> Result<FarmPosition> {
        let manager = FarmManager::new(&self.client);
        manager.get_position(wallet, farm).await
    }

    /// Parse LP token to get underlying assets.
    pub async fn parse_lp_token(&self, lp_mint: &Pubkey) -> Result<LpTokenInfo> {
        let parser = LpTokenParser::new(&self.client);
        parser.parse(lp_mint).await
    }
}

/// Identify which DEX a pool belongs to.
pub fn identify_dex(pool_program: &Pubkey) -> Option<Dex> {
    if pool_program == &*programs::RAYDIUM_AMM_V4 {
        Some(Dex::RaydiumAmm)
    } else if pool_program == &*programs::RAYDIUM_CLMM {
        Some(Dex::RaydiumClmm)
    } else if pool_program == &*programs::ORCA_WHIRLPOOL {
        Some(Dex::OrcaWhirlpool)
    } else if pool_program == &*programs::METEORA_DLMM {
        Some(Dex::MeteoraDlmm)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dex {
    RaydiumAmm,
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDlmm,
    Unknown,
}

impl Dex {
    pub fn name(&self) -> &'static str {
        match self {
            Dex::RaydiumAmm => "Raydium AMM",
            Dex::RaydiumClmm => "Raydium CLMM",
            Dex::OrcaWhirlpool => "Orca Whirlpool",
            Dex::MeteoraDlmm => "Meteora DLMM",
            Dex::Unknown => "Unknown",
        }
    }

    pub fn supports_concentrated_liquidity(&self) -> bool {
        matches!(self, Dex::RaydiumClmm | Dex::OrcaWhirlpool | Dex::MeteoraDlmm)
    }
}
