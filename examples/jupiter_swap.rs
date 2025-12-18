//! Jupiter swap example
//!
//! This example demonstrates how to use the Jupiter integration
//! to swap tokens on Solana with best-price routing.
//!
//! Run with: cargo run --example jupiter_swap --features jupiter

use solana_pipkit::jupiter::{JupiterClient, SwapConfig};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load wallet from default Solana CLI location or environment
    let wallet = load_wallet()?;
    println!("Using wallet: {}", wallet.pubkey());

    // Use devnet or mainnet based on environment
    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    
    println!("Using RPC: {}", rpc_url);

    // Create Jupiter client
    let jupiter = JupiterClient::new(&rpc_url);

    // Example 1: Get a quote for swapping 0.1 SOL to USDC
    println!("\n=== Getting Quote: 0.1 SOL → USDC ===");
    let sol_amount = LAMPORTS_PER_SOL / 10; // 0.1 SOL
    
    let quote = jupiter
        .get_quote(
            JupiterClient::SOL_MINT,
            JupiterClient::USDC_MINT,
            sol_amount,
            50, // 0.5% slippage
        )
        .await?;

    println!("Input: {} lamports ({} SOL)", quote.in_amount, quote.in_amount as f64 / LAMPORTS_PER_SOL as f64);
    println!("Output: {} USDC units ({} USDC)", quote.out_amount, quote.out_amount as f64 / 1_000_000.0);
    println!("Price impact: {}%", quote.price_impact_pct);
    println!("Slippage: {} bps", quote.slippage_bps);
    
    // Show route
    println!("\nRoute:");
    for step in &quote.route_plan {
        let label = step.swap_info.label.as_deref().unwrap_or("Unknown");
        println!("  → {} ({}%)", label, step.percent);
    }

    // Example 2: Get price for 1 USDC → SOL
    println!("\n=== Price Check: 1 USDC → SOL ===");
    let usdc_amount = 1_000_000; // 1 USDC
    
    let sol_output = jupiter
        .get_price(JupiterClient::USDC_MINT, JupiterClient::SOL_MINT, usdc_amount)
        .await?;
    
    println!("1 USDC = {} SOL", sol_output as f64 / LAMPORTS_PER_SOL as f64);

    // Example 3: Get route labels
    println!("\n=== Route Discovery ===");
    let labels = jupiter
        .get_route_labels(JupiterClient::SOL_MINT, JupiterClient::USDC_MINT, sol_amount)
        .await?;
    
    println!("DEXs used: {}", labels.join(" → "));

    // Example 4: Execute a swap (commented out for safety)
    // Uncomment to actually execute a swap
    /*
    println!("\n=== Executing Swap ===");
    let config = SwapConfig::with_slippage(100) // 1% slippage for safety
        .with_priority_fee(5000); // Priority fee
    
    let signature = jupiter
        .swap_with_config(&wallet, quote, config)
        .await?;
    
    println!("Swap executed!");
    println!("Signature: {}", signature);
    println!("View on Solscan: https://solscan.io/tx/{}", signature);
    */

    // Example 5: Simple swap helper (commented out for safety)
    /*
    let sig = jupiter.simple_swap(
        &wallet,
        JupiterClient::SOL_MINT,
        JupiterClient::USDC_MINT,
        LAMPORTS_PER_SOL / 100, // 0.01 SOL
        50, // 0.5% slippage
    ).await?;
    println!("Simple swap executed: {}", sig);
    */

    println!("\n✅ Jupiter integration working!");
    println!("Uncomment the swap examples to execute actual trades.");

    Ok(())
}

fn load_wallet() -> Result<Keypair, Box<dyn std::error::Error>> {
    // Try environment variable first
    if let Ok(key) = env::var("SOLANA_PRIVATE_KEY") {
        let bytes: Vec<u8> = serde_json::from_str(&key)?;
        return Ok(Keypair::from_bytes(&bytes)?);
    }

    // Try default Solana CLI keypair location
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
    let keypair_path = format!("{}/.config/solana/id.json", home);
    
    if std::path::Path::new(&keypair_path).exists() {
        return Ok(read_keypair_file(&keypair_path)
            .map_err(|e| format!("Failed to read keypair: {}", e))?);
    }

    // Generate a new keypair for testing (no funds)
    println!("⚠️  No wallet found, generating temporary keypair (no funds)");
    Ok(Keypair::new())
}
