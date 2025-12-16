<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <source media="(prefers-color-scheme: light)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <img src="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1" style="width: 40%; height: 40%;" alt="Solana crab logo">
</picture>


https://img.shields.io/badge/📖 docs-docs.rs-00FFA3.svg  
https://img.shields.io/badge/docs-API%20Reference-00FFA3.svg  
https://img.shields.io/crates/v/solana-pipkit.svg?color=00FFA3
 
https://img.shields.io/crates/d/solana-pipkit.svg?color=00FFA3


https://img.shields.io/discord/149207?color=%239945FF&label=Discord&logo=discord&logoColor=white
 
GitHub stars


https://img.shields.io/badge/built%20on-Solana-00FFA3.svg?logo=solana&logoColor=white
 
https://img.shields.io/badge/built%20with-Rust-00FFA3.svg?logo=rust&logoColor=white
 
https://img.shields.io/badge/built%20by-ARK%20Technologies-00FFA3.svg


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
solana-pipkit = "0.1"  # Replace with the latest version
Or install the latest version with:
Bashcargo add solana-pipkit
Simple example
Example: Recovering rent from empty accounts
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signer::Signer, signature::read_keypair_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_keypair_file("~/.config/solana/id.json")?;
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





Rust logo
      
Solana crab logo


Built by Noah Michél at ARK Technologies

I've reformatted the badges to match the style of the Rig README you referenced—grouped with line breaks, non-breaking spaces ( ), and consistent coloring using Solana's vibrant surge green (#00FFA3). Added logos where possible (Solana, Rust, Discord), included crates.io version/downloads badges (they'll show actual data once published), and a Discord badge for the official Solana Tech server. This should give it a polished, professional look similar to Rig! Let me kno
