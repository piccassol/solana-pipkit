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
  <a href="https://crates.io/crates/solana-pipkit">
    <img src="https://img.shields.io/crates/v/solana-pipkit.svg?style=flat&color=00FFA3" alt="Crates.io" />
  </a>
  <a href="https://www.notion.so/Solana-pipkit-Doc-001-2cb9a71542d480128dabe02e7d58026b">
    <img src="https://img.shields.io/badge/docs-Notion-00FFA3.svg" alt="Documentation" />
  </a>
  <a href="https://github.com/piccassol/solana-pipkit">
    <img src="https://img.shields.io/github/stars/piccassol/solana-pipkit?style=social" />
  </a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/built%20on-Solana-00FFA3.svg?logo=solana" />
  <img src="https://img.shields.io/badge/built%20with-Rust-00FFA3.svg?logo=rust" />
  <img src="https://img.shields.io/badge/built%20by-ARK%20Technologies-00FFA3.svg" />
</p>

<p align="center">
  <a href="https://crates.io/crates/solana-pipkit">Crates.io</a>
  &nbsp;•&nbsp;
  <a href="https://www.notion.so/Solana-pipkit-Doc-001-2cb9a71542d480128dabe02e7d58026b">Documentation</a>
  &nbsp;•&nbsp;
  <a href="https://docs.solana.com">Solana Docs</a>
  &nbsp;•&nbsp;
  <a href="https://github.com/piccassol/solana-pipkit/issues/new">Contribute</a>
  &nbsp;•&nbsp;
  <a href="./examples">Examples</a>
</p>

---

## What is solana-pipkit?

**solana-pipkit** is a Rust utility crate designed to streamline common tasks in **Solana program and client development**.
It focuses on ergonomics, safety, and reusable patterns for production-grade Solana workflows.

---

## Installation

```sh
cargo add solana-pipkit
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
solana-pipkit = "1.3.0"

# Enable high-performance trading features
solana-pipkit = { version = "1.3.0", features = ["speed"] }
```

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

- **Safety Protocol**
  Client-side validation to prevent costly transaction mistakes

- **Speed Module** *(v1.3.0 "Lightning")*
  High-performance execution for trading agents with connection pooling, blockhash caching, and optimized swaps

---

## Speed Module

Built for trading bots and agents that need minimal latency. The speed module provides optimized RPC clients, instant transaction building, and fast swap execution.

```rust
use solana_pipkit::speed::prelude::*;
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Multi-endpoint RPC with automatic failover
    let rpc = FastRpcClient::new(vec![
        "https://api.mainnet-beta.solana.com".into(),
        "https://solana-api.projectserum.com".into(),
    ]);

    // Benchmark and use fastest endpoint
    rpc.set_strategy(LoadBalanceStrategy::LeastLatency);
    let health = rpc.benchmark_endpoints().await;
    println!("Fastest: {} ({}ms)", health[0].url, health[0].latency_ms);

    // Fast swap execution
    let payer = Keypair::new();
    let executor = SwapExecutor::new(rpc, payer);

    let result = executor.swap_fast(
        &mints::USDC,
        &mints::SOL,
        1_000_000,  // 1 USDC
        SwapConfig::fast(),  // High priority, direct routes
    ).await?;

    println!("Swapped in {}ms", result.execution_time_ms);
    Ok(())
}
```

**Performance targets:**
- Cached blockhash retrieval: **0ms**
- ATA computation: **<50us**
- Quote to execution: **<500ms**

---

## Safety Features

Crypto losses from typos and decimal errors are preventable. The safety module catches mistakes before they hit the chain.

```rust
use solana_pipkit::prelude::*;

let protocol = SafetyProtocol::new()
    .token_price(100.0)  // $100/SOL for USD calculations
    .large_amount_threshold(1000.0);

let report = protocol.validate_offline(
    &sender,
    &recipient,
    amount_lamports,
    9,  // SOL decimals
    balance,
);

if !report.approved {
    println!("Blocked: {:?}", report.blockers);
    return Err("Transaction failed safety check");
}

if report.requires_confirmation {
    println!("Warnings: {:?}", report.warnings);
    // Prompt user to confirm
}
```

**What it prevents:**
- Address typos (invalid base58, wrong length)
- Sending to yourself accidentally
- Draining entire balance (>90% warning)
- Decimal/magnitude errors (1000 vs 1.000)
- Large transfers without confirmation (>$1000 USD)

**Risk levels:** `Low` | `Medium` | `High` | `Critical`

See [`examples/full_safety_demo.rs`](./examples/full_safety_demo.rs) for complete usage.

---

## Quick Example

Reclaiming rent from empty accounts:

```rust
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
```

More examples are available in the [examples/](./examples) directory.

---

## Project Status

> **Early-stage utility crate** - APIs may evolve and breaking changes can occur as the toolkit matures.

Fast chains deserve safe, expressive tooling. **solana-pipkit** exists to remove boilerplate, reduce footguns, and let you focus on protocol logic.

---

## Contributing

Contributions are welcome and appreciated.

- Open an issue for bugs, ideas, or discussion
- Submit a PR for improvements or new helpers
- Keep APIs ergonomic, composable, and Solana-native

[Get started](https://github.com/piccassol/solana-pipkit/issues/new)

---

## License

This project is licensed under the MIT License.

---

<p align="center">
  <strong>Ark Technologies AI</strong><br/>
  Engineering high-performance infrastructure and developer tooling<br/>
  for next-generation decentralized systems.
</p>

<p align="center">
  Built by <strong>Noah Michél</strong><br/>
  © Ark Technologies AI
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Built%20by-ARK%20Technologies-00FFA3.svg" />
  <img src="https://img.shields.io/badge/Rust-%F0%9F%A6%80-000000.svg?logo=rust" />
  <img src="https://img.shields.io/badge/Solana-%E2%9A%A1-00FFA3.svg?logo=solana" />
</p>

<p align="center">
  If this toolkit helps your Solana development, consider starring the repo.
</p>
