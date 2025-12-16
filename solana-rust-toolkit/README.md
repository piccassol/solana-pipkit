# Solana Rust Toolkit

A collection of Rust utilities for Solana development — rent recovery, token operations, PDA helpers, and common program patterns.

## Features

- **Rent Cleaner** — Reclaim SOL from empty accounts
- **Token Utils** — SPL token helpers (burn, transfer, close)
- **PDA Helpers** — Simplified PDA derivation and validation
- **Account Utils** — Common account operations and validation
- **Anchor Patterns** — Reusable Anchor program structures

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solana-rust-toolkit = { git = "https://github.com/piccassol/solana-rust-toolkit" }
```

Or for specific modules:

```toml
[dependencies]
solana-sdk = "1.18"
solana-client = "1.18"
spl-token = "4.0"
anchor-lang = "0.29"
```

## Quick Start

### Rent Cleaner

```rust
use solana_rust_toolkit::rent_cleaner::RentCleaner;

let cleaner = RentCleaner::new(rpc_url, keypair);
let recovered = cleaner.clean_empty_accounts().await?;
println!("Recovered {} SOL", recovered);
```

### Token Operations

```rust
use solana_rust_toolkit::token_utils::{burn_tokens, close_token_account};

// Burn tokens
burn_tokens(&client, &payer, &mint, &token_account, amount).await?;

// Close empty token account and reclaim rent
close_token_account(&client, &payer, &token_account).await?;
```

### PDA Helpers

```rust
use solana_rust_toolkit::pda::{derive_pda, find_metadata_pda};

// Derive custom PDA
let (pda, bump) = derive_pda(&[b"vault", user.as_ref()], &program_id);

// Find token metadata PDA
let metadata_pda = find_metadata_pda(&mint);
```

## Module Reference

### `rent_cleaner`
Scan and close rent-exempt accounts with zero balance.

### `token_utils`
SPL token operations: mint, burn, transfer, freeze, close accounts.

### `pda`
PDA derivation helpers for common Solana programs (Token, Metadata, Associated Token).

### `account_utils`
Account validation, data parsing, and common checks.

### `programs`
Anchor program templates and patterns.

## Examples

See the `/examples` directory for complete working examples:

```bash
# Run rent cleaner
cargo run --example rent_cleaner

# Token burn example
cargo run --example token_burn

# PDA derivation
cargo run --example pda_derive
```

## Development

```bash
# Build
cargo build

# Test
cargo test

# Format
cargo fmt

# Lint
cargo clippy
```

## License

MIT

---

Built by [Noah Michél](https://github.com/piccassol) @ [ARK Technologies](https://arktechnologies.ai)
