<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <source media="(prefers-color-scheme: light)" srcset="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1">
    <img src="https://imgs.search.brave.com/sZXOOoqvGeIvFDgPAfF2_lVUWTqMv49aGLRmYsL6vj8/rs:fit:500:0:1:0/g:ce/aHR0cHM6Ly9zb2xh/bmEuY29tL19uZXh0/L2ltYWdlP3VybD1o/dHRwczovL2Nkbi5i/dWlsZGVyLmlvL2Fw/aS92MS9pbWFnZS9h/c3NldHMlMkZjZTBj/NzMyM2E5N2E0ZDkx/YmQwYmFhNzQ5MGVj/OTEzOSUyRjA5OWQ2/MDcyMzNhMDQ3OWJi/NGY1MDMyMGExMmNk/YmZlJnc9MTA4MCZx/PTc1" style="width: 40%; height: 40%;" alt="Solana crab logo">
</picture>


https://img.shields.io/badge/📖 docs-docs.rs-00FFA3.svg  
https://img.shields.io/badge/docs-API%20Reference-00FFA3.svg  


https://img.shields.io/discord/149207?color=%237289DA&label=Discord&logo=discord&logoColor=white
 
GitHub stars


https://img.shields.io/badge/built%20on-Solana-00FFA3.svg?logo=solana
 
https://img.shields.io/badge/built%20with-Rust-00FFA3.svg?logo=rust
 
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
High-level features
Get Started
Simple example

Examples

What is solana-pipkit?
solana-pipkit is a Rust utility crate designed to streamline common tasks in Solana program and client development. It provides ergonomic helpers for rent reclamation, SPL token management, PDA derivation, account validation, and reusable Anchor patterns.
Detailed documentation is available in the API Reference.
High-level features

Rent Recovery: Efficiently reclaim lamports from dormant or empty accounts
SPL Token Helpers: Simplified operations for burning, transferring, and closing token accounts
PDA Management: Convenient derivation, seeding, and validation utilities (including Metaplex metadata PDAs)
Account Utilities: Common validation patterns and deserialization helpers
Anchor Reusables: Macros and structures for cleaner, more maintainable Anchor programs

Get Started
Add to your project via git (until published on crates.io):
toml[dependencies]
solana-pipkit = { git = "https://github.com/piccassol/solana-pipkit" }
Simple example
Reclaiming rent from empty accounts:
Rustuse solana_pipkit::rent::RentCleaner;
use solana_sdk::{signature::read_keypair_file, signer::Signer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_keypair_file("~/.config/solana/id.json")?;
    let rpc_url = "https://api.mainnet-beta.solana.com";

    let cleaner = RentCleaner::new(rpc_url, keypair.pubkey(), keypair);

    let reclaimed = cleaner.clean_empty_accounts().await?;
    println!("Reclaimed {} lamports", reclaimed);

    Ok(())
}
Find more examples in the examples/ directory.
License
Licensed under the MIT License.





Rust logo
      
Solana crab logo


Built by Noah Michél at ARK Technologies
