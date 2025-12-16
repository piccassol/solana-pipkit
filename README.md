<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <source media="(prefers-color-scheme: light)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <img src="https://cryptologos.cc/logos/solana-sol-logo.png?v=029" width="160" alt="Solana logo">
  </picture>
</p>

<h1 align="center">solana-pipkit</h1>

<p align="center">
  A pragmatic Rust toolkit for Solana program & client development
</p>

<p align="center">
  <a href="https://docs.rs/solana-pipkit">
    <img src="https://img.shields.io/badge/📖%20docs-docs.rs-00FFA3.svg" />
  </a>
  <a href="https://docs.rs/solana-pipkit">
    <img src="https://img.shields.io/badge/docs-API%20Reference-00FFA3.svg" />
  </a>
  <a href="https://github.com/piccassol/solana-pipkit">
    <img src="https://img.shields.io/github/stars/piccassol/solana-pipkit?style=social" />
  </a>
</p>

<p align="center">
  <a href="https://discord.gg/149207">
    <img src="https://img.shields.io/discord/149207?color=%237289DA&label=Discord&logo=discord&logoColor=white" />
  </a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/built%20on-Solana-00FFA3.svg?logo=solana" />
  <img src="https://img.shields.io/badge/built%20with-Rust-00FFA3.svg?logo=rust" />
  <img src="https://img.shields.io/badge/built%20by-ARK%20Technologies-00FFA3.svg" />
</p>

<p align="center">
  <a href="https://docs.rs/solana-pipkit">📖 API Reference</a>
  &nbsp;•&nbsp;
  <a href="https://docs.solana.com">🔗 Solana Docs</a>
  &nbsp;•&nbsp;
  <a href="https://github.com/piccassol/solana-pipkit/issues/new">🤝 Contribute</a>
  &nbsp;•&nbsp;
  <a href="./examples">✨ Examples</a>
</p>

---

## What is solana-pipkit?

**solana-pipkit** is a Rust utility crate designed to streamline common tasks in **Solana program and client development**.  
It focuses on ergonomics, safety, and reusable patterns for production-grade Solana workflows.

Detailed documentation is available in the **API Reference**.

---

## High-level features

- **Rent Recovery**  
  Efficiently reclaim lamports from dormant or empty accounts

- **SPL Token Helpers**  
  Simplified helpers for burning, transferring, and closing token accounts

- **PDA Management**  
  Utilities for derivation, seeding, and validation (including Metaplex metadata PDAs)

- **Account Utilities**  
  Common validation patterns and deserialization helpers

- **Anchor Reusables**  
  Macros and shared structures for cleaner, more maintainable Anchor programs

---

## Get Started

Add to your project via git (until published on crates.io):

```toml
[dependencies]
solana-pipkit = { git = "https://github.com/piccassol/solana-pipkit" }
Simple example
Reclaiming rent from empty accounts:

rust
Copy code
use solana_pipkit::rent::RentCleaner;
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
More examples are available in the examples/ directory.

⚠️ Note
This is an early-stage utility crate. APIs may evolve, and breaking changes can occur as the toolkit matures.

License
MIT © ARK Technologies
Built by Noah Michél at ARK Technologies
