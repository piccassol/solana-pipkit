<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.vectorstock.com/i/1000v/79/42/solana-logo-coin-icon-isolated-vector-43697942.jpg">
    <source media="(prefers-color-scheme: light)" srcset="https://upload.wikimedia.org/wikipedia/commons/3/34/Solana_cryptocurrency_two.jpg">
    <img src="https://upload.wikimedia.org/wikipedia/commons/3/34/Solana_cryptocurrency_two.jpg" style="width: 40%; height: 40%;" alt="Solana logo">
</picture>


https://img.shields.io/github/stars/piccassol/solana-pipkit?style=social
 
https://img.shields.io/github/forks/piccassol/solana-pipkit?style=social


https://img.shields.io/crates/v/solana-pipkit.svg?color=dea584
 
https://img.shields.io/crates/d/solana-pipkit.svg?color=dea584
 
https://img.shields.io/badge/docs-API%20Reference-dea584.svg


https://img.shields.io/badge/built%20on-Solana-dea584.svg
 
https://img.shields.io/badge/built%20with-Rust-dea584.svg?logo=rust


https://img.shields.io/badge/built%20by-ARK%20Technologies-dea584


[📖 API Reference](https://docs.rs/solana-pipkit)
  •  
[🔗 Solana Docs](https://docs.solana.com)
  •  
[🤝 Contribute](https://github.com/piccassol/solana-pipkit/issues/new)
  •  
[✨ Examples](./examples)

✨ If you find this toolkit helpful for your Solana development, please consider starring the repo!
[!NOTE]
This is an early-stage utility crate. Features may evolve, and breaking changes could occur as the toolkit matures.
Table of contents

Table of contents
What is solana-pipkit?
Features
Get Started
Simple example

Examples

What is solana-pipkit?
solana-pipkit is a Rust crate providing a collection of practical utilities for Solana program development. It simplifies common tasks like rent recovery, SPL token operations, PDA management, and reusable Anchor patterns.
More details can be found in the API Reference.
Features

Rent Cleaner: Reclaim SOL from empty or closed accounts
Token Utils: Helpers for burning tokens, transferring, and closing token accounts
PDA Helpers: Easy PDA derivation, validation, and common finders (e.g., metadata PDA)
Account Utils: Validation and common account operations
Anchor Patterns: Reusable structures and macros for Anchor-based programs

Get Started
Add the crate to your Cargo.toml:
toml[dependencies]
solana-pipkit = { git = "https://github.com/piccassol/solana-pipkit" }
(Once published to crates.io, use version instead of git.)
Simple example
Example: Recovering rent from empty accounts
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signer::Signer, signature::read_keypair_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_key_pair_file("~/.config/solana/id.json")?;
    let rpc_url = "https://api.mainnet-beta.solana.com";

    let cleaner = RentCleaner::new(rpc_url, keypair.pubkey(), keypair);

    let reclaimed = cleaner.clean_empty_accounts().await?;
    println!("Reclaimed {} lamports", reclaimed);

    Ok(())
}
More examples are available in the examples/ directory:

Rent cleaning
Token burning and closing
PDA derivation

License
This project is licensed under the MIT License.





Built with Rust
   
Built on Solana


Built by Noah Michél at ARK Technologies
