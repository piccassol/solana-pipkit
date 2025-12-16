//! Example: Burn tokens and close the account.

use solana_rust_toolkit::token_utils::TokenClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment
    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| shellexpand::tilde("~/.config/solana/id.json").to_string());

    let keypair_data = std::fs::read_to_string(&keypair_path)?;
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
    let payer = Keypair::from_bytes(&keypair_bytes)?;

    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    // Get mint and token account from args or environment
    let mint_str = env::var("MINT").expect("MINT environment variable required");
    let token_account_str = env::var("TOKEN_ACCOUNT")
        .expect("TOKEN_ACCOUNT environment variable required");
    let amount: u64 = env::var("AMOUNT")
        .expect("AMOUNT environment variable required")
        .parse()?;

    let mint = Pubkey::from_str(&mint_str)?;
    let token_account = Pubkey::from_str(&token_account_str)?;

    println!("Configuration:");
    println!("  Wallet: {}", payer.pubkey());
    println!("  Mint: {}", mint);
    println!("  Token Account: {}", token_account);
    println!("  Amount to burn: {}", amount);
    println!();

    // Create token client
    let client = TokenClient::new(&rpc_url, payer);

    // Get current balance
    let balance = client.get_balance(&token_account).await?;
    println!("Current balance: {}", balance);

    if balance < amount {
        println!("Error: Insufficient balance to burn {} tokens", amount);
        return Ok(());
    }

    // Burn tokens
    println!("\nBurning {} tokens...", amount);
    client.burn(&mint, &token_account, amount).await?;
    println!("Burn successful!");

    // Check if account is now empty
    let new_balance = client.get_balance(&token_account).await?;
    println!("New balance: {}", new_balance);

    if new_balance == 0 {
        println!("\nAccount is empty. Closing to recover rent...");
        let recovered = client.close_account(&token_account).await?;
        println!(
            "Closed account and recovered {} lamports ({:.6} SOL)",
            recovered,
            recovered as f64 / 1_000_000_000.0
        );
    }

    Ok(())
}
