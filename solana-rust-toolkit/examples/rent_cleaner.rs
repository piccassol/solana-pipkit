//! Example: Clean empty token accounts and recover SOL.

use solana_rust_toolkit::rent_cleaner::{RentCleaner, RentCleanerConfig};
use solana_sdk::signature::Keypair;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load keypair from file or environment
    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| shellexpand::tilde("~/.config/solana/id.json").to_string());

    let keypair_data = std::fs::read_to_string(&keypair_path)?;
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
    let payer = Keypair::from_bytes(&keypair_bytes)?;

    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    println!("Wallet: {}", payer.pubkey());
    println!("RPC: {}", rpc_url);

    // Create cleaner with dry run mode first
    let config = RentCleanerConfig {
        dry_run: true,
        ..Default::default()
    };

    let cleaner = RentCleaner::with_config(&rpc_url, payer, config);

    // Find empty accounts
    println!("\nScanning for empty token accounts...");
    let accounts = cleaner.find_empty_token_accounts().await?;

    if accounts.is_empty() {
        println!("No empty token accounts found.");
        return Ok(());
    }

    println!("\nFound {} empty token accounts:", accounts.len());
    let mut total_recoverable: u64 = 0;

    for account in &accounts {
        println!(
            "  {} - {} lamports ({:.6} SOL)",
            account.address,
            account.lamports,
            account.lamports as f64 / 1_000_000_000.0
        );
        total_recoverable += account.lamports;
    }

    println!(
        "\nTotal recoverable: {} lamports ({:.6} SOL)",
        total_recoverable,
        total_recoverable as f64 / 1_000_000_000.0
    );

    // Prompt to continue (in a real app)
    println!("\nTo actually close these accounts, run with DRY_RUN=false");

    Ok(())
}
