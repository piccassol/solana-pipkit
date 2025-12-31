<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <source media="(prefers-color-scheme: light)" srcset="https://cryptologos.cc/logos/solana-sol-logo.png?v=029">
    <img src="https://cryptologos.cc/logos/solana-sol-logo.png?v=029" width="160" alt="Solana logo">
  </picture>
</p>

<h1 align="center">solana-pipkit</h1>

<p align="center">
  A pragmatic Rust toolkit for Solana program and client development
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

As of version **2.0.0**, solana-pipkit represents a stable, cohesive client-side framework for building trading systems, wallets, bots, analytics pipelines, and developer tooling on Solana.

---

## Installation

```sh
cargo add solana-pipkit
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
solana-pipkit = "2.0.0"

# Enable high-performance trading features
solana-pipkit = { version = "2.0.0", features = ["speed"] }
```

---

## High-level features

- **Rent Recovery**
  Efficiently reclaim lamports from dormant or empty accounts

- **SPL Token Helpers**
  Simplified helpers for burning, transferring, and closing token accounts

- **PDA Management**
  Utilities for derivation, seeding, and validation including Metaplex metadata PDAs

- **Account Utilities**
  Common validation patterns, deserialization helpers, and account graph traversal

- **Transaction Batching**
  Fluent builders and batch executors for reliable multi-transaction workflows

- **Anchor Reusables**
  Shared structures and helpers for cleaner, more maintainable Anchor programs

- **Safety Protocol**
  Client-side validation to prevent costly transaction mistakes before submission

- **Transaction Simulation**
  Pre-flight simulation for previewing balance changes, compute usage, and failure risks

- **Program and NFT Safety**
  Detection of risky programs, upgrade authorities, and suspicious or fake NFTs

- **MEV Protection**
  Sandwich risk analysis, priority fee recommendations, and Jito bundle support

- **Speed Module** (introduced in v1.3.0, consolidated in v2.0.0)
  High-performance execution for trading agents with connection pooling, blockhash caching, and optimized swaps

- **Analytics Module** (v2.0.0)
  Wallet profiling, PnL analysis, and behavioral classification

- **DeFi Module** (v2.0.0)
  Pool inspection, LP position tracking, and farming position analysis

---

## Speed Module

Built for trading bots and agents that need minimal latency. The speed module provides optimized RPC clients, instant transaction building, and fast swap execution.

```rust
use solana_pipkit::speed::prelude::*;
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = FastRpcClient::new(vec![
        "https://api.mainnet-beta.solana.com".into(),
        "https://solana-api.projectserum.com".into(),
    ]);

    rpc.set_strategy(LoadBalanceStrategy::LeastLatency);
    let health = rpc.benchmark_endpoints().await;

    let payer = Keypair::new();
    let executor = SwapExecutor::new(rpc, payer);

    let result = executor.swap_fast(
        &mints::USDC,
        &mints::SOL,
        1_000_000,
        SwapConfig::fast(),
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

## Transaction Simulation

Before sending a transaction, solana-pipkit allows you to simulate execution locally or via RPC.

Simulation provides insight into balance changes, compute unit usage, instruction failures, and swap outcomes without committing state on-chain.

Simulation is a first-class component in version 2.0.0 and integrates directly with safety analysis to block or warn on unsafe outcomes.

---

## Project Status

Stable major release. Version 2.0.0 marks a consolidated and production-ready API surface. Backwards compatibility will be preserved within the 2.x series.

Fast chains deserve safe, expressive tooling. solana-pipkit exists to reduce boilerplate, eliminate common footguns, and provide a reliable foundation for Solana client-side infrastructure.

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
