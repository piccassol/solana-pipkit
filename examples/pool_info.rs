//! Example: Get information about a liquidity pool.
//!
//! Run with: cargo run --example pool_info --features defi

#[cfg(feature = "defi")]
use solana_client::rpc_client::RpcClient;
#[cfg(feature = "defi")]
use solana_sdk::pubkey::Pubkey;
#[cfg(feature = "defi")]
use std::str::FromStr;
#[cfg(feature = "defi")]
use solana_pipkit::defi::{DeFi, DeFiConfig, calculate_constant_product_swap};

fn main() {
    #[cfg(feature = "defi")]
    {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let client = RpcClient::new(&rpc_url);
        
        // Example: SOL-USDC Raydium pool
        let pool = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2")
            .expect("Invalid pubkey");
        
        let config = DeFiConfig {
            default_slippage_bps: 50,
            use_jito: false,
            priority_fee: 10000,
        };
        
        let defi = DeFi::with_config(client, config);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        match rt.block_on(defi.get_pool_info(&pool)) {
            Ok(info) => {
                println!("Pool: {}", info.address);
                println!("DEX: {}", info.dex.name());
                println!("");
                println!("Token A: {}", info.token_a.mint);
                println!("  Reserve: {}", info.reserve_a);
                println!("");
                println!("Token B: {}", info.token_b.mint);
                println!("  Reserve: {}", info.reserve_b);
                println!("");
                println!("Fee Rate: {} bps", info.fee_rate_bps);
                println!("LP Supply: {}", info.lp_supply);
                
                if let Some(tvl) = info.tvl_usd {
                    println!("TVL: ${:.2}", tvl);
                }
                
                // Simulate a swap
                let swap_amount = 1_000_000_000; // 1 SOL in lamports
                let result = calculate_constant_product_swap(
                    info.reserve_a,
                    info.reserve_b,
                    swap_amount,
                    info.fee_rate_bps,
                );
                
                println!("");
                println!("Swap Simulation (1 SOL -> USDC):");
                println!("  Output: {} USDC", result.amount_out as f64 / 1_000_000.0);
                println!("  Fee: {} lamports", result.fee_amount);
                println!("  Price Impact: {:.4}%", result.price_impact_pct);
            }
            Err(e) => eprintln!("Error getting pool info: {}", e),
        }
    }
    
    #[cfg(not(feature = "defi"))]
    {
        println!("Enable the 'defi' feature to run this example:");
        println!("  cargo run --example pool_info --features defi");
    }
}
