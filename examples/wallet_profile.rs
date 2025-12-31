//! Example: Profile a wallet to detect whale status, activity patterns, and classification.
//!
//! Run with: cargo run --example wallet_profile --features analytics

#[cfg(feature = "analytics")]
use solana_client::rpc_client::RpcClient;
#[cfg(feature = "analytics")]
use solana_sdk::pubkey::Pubkey;
#[cfg(feature = "analytics")]
use std::str::FromStr;
#[cfg(feature = "analytics")]
use solana_pipkit::analytics::{Analytics, AnalyticsConfig, WalletClassification};

fn main() {
    #[cfg(feature = "analytics")]
    {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let client = RpcClient::new(&rpc_url);
        
        // Example wallet - replace with actual address
        let wallet = Pubkey::from_str("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg")
            .expect("Invalid pubkey");
        
        let config = AnalyticsConfig {
            whale_threshold_sol: 1000.0,
            major_holder_threshold_pct: 1.0,
            pattern_lookback: 100,
            track_smart_money: true,
        };
        
        let analytics = Analytics::with_config(client, config);
        
        // Use tokio runtime for async
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        match rt.block_on(analytics.profile_wallet(&wallet)) {
            Ok(profile) => {
                println!("Wallet Profile: {}", profile.address);
                println!("  SOL Balance: {:.2}", profile.sol_balance);
                println!("  Token Count: {}", profile.token_count);
                println!("  Transaction Count: {}", profile.transaction_count);
                println!("  Activity Level: {:?}", profile.activity_level);
                println!("  Classification: {:?}", profile.classification);
                
                if !profile.flags.is_empty() {
                    println!("  Flags:");
                    for flag in &profile.flags {
                        println!("    - {:?}", flag);
                    }
                }
                
                match profile.classification {
                    WalletClassification::Whale => println!("\n  This is a whale wallet."),
                    WalletClassification::SmartMoney => println!("\n  Known smart money address."),
                    WalletClassification::Bot => println!("\n  Likely automated trading."),
                    _ => {}
                }
            }
            Err(e) => eprintln!("Error profiling wallet: {}", e),
        }
    }
    
    #[cfg(not(feature = "analytics"))]
    {
        println!("Enable the 'analytics' feature to run this example:");
        println!("  cargo run --example wallet_profile --features analytics");
    }
}
