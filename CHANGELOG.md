# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2024-12-27

### Fixed

- docs: Fix README installation instructions

## [1.0.0] - 2024-12-27

### Added

- **Client-Side Safety Protocol** (`safety` module)
  - `AddressVerifier` for address validation and typo detection
  - `AmountValidator` for decimal conversion and magnitude error detection
  - `SafetyProtocol` orchestrator for unified transfer validation
  - `SafetyReport` with risk levels (Low, Medium, High, Critical)
  - Full balance warning (>90% of balance)
  - Large amount confirmation (>$1000 USD threshold)
  - Strict mode for zero-tolerance validation

- **New Examples**
  - `address_verification.rs` - Address validation demo
  - `full_safety_demo.rs` - Complete safety protocol walkthrough

### Changed

- Updated to v1.0.0 stable release
- Added safety types to prelude exports

## [0.2.0] - 2024-12-21

### Added

- **Transaction Batching Utilities** (`transaction` module)
  - `TransactionBuilder` for fluent transaction construction
  - `TransactionConfig` for compute units, priority fees, and retry settings
  - `BatchExecutor` for sequential batch processing
  - `ParallelBatchExecutor` for concurrent transaction execution
  - Transaction size estimation utilities
  - Automatic retry logic with configurable attempts

- **Account Graph Traversal** (`account_graph` module)
  - `AccountGraph` for tracking account relationships
  - `AccountGraphBuilder` for constructing graphs from on-chain data
  - `AccountNode` with type classification (System, Token, Mint, Metadata, etc.)
  - Graph traversal utilities (find reachable, find reaching)
  - Token account grouping by mint or wallet
  - Utility functions for finding closeable accounts

- **Advanced Rent Recovery Strategies** (`rent_cleaner` module)
  - `AdvancedRentCleaner` with configurable strategies
  - `CleanupStrategy` enum: EmptyOnly, BelowDustThreshold, BurnAndClose, AggregateAndClose
  - `CleanupPriority` enum: HighValue, QuickWins, ByMint, OldestFirst
  - `AdvancedCleanupConfig` with mint filtering (include/exclude)
  - `RecoveryBreakdown` for detailed analysis
  - Batch processing with automatic fallback to individual operations

- **Expanded Anchor Helpers** (`anchor_helpers` module)
  - Proper discriminator calculation for accounts and instructions
  - `CpiInstructionBuilder` for building CPI calls fluently
  - `RemainingAccountsBuilder` for Anchor remaining accounts
  - System Program CPI helpers (`system_cpi`)
  - SPL Token CPI helpers (`token_cpi`)
  - Associated Token Account CPI helpers (`ata_cpi`)
  - Account validation utilities (discriminator, owner, data length, rent exempt)
  - Serialization helpers for Anchor instruction data
  - Common account size constants

- **Enhanced Error Handling**
  - `BatchError` variant for batch operation failures
  - `GraphError` variant for account graph issues
  - `ConfigError` variant for configuration problems
  - `Timeout` variant for operation timeouts
  - Helper methods: `is_retryable()`, `transaction()`, `custom()`

- **Improved Documentation**
  - Comprehensive module-level documentation
  - Doc comments on all public types and functions
  - Quick start examples in lib.rs
  - Feature flag documentation

### Changed

- Updated version to 0.2.0
- Improved `prelude` module with all new exports
- Added `futures` dependency for parallel execution
- Moved `bincode` from optional to required dependency
- Enhanced Cargo.toml metadata for crates.io

### Fixed

- N/A (new features only)

## [0.1.0] - 2024-12-20

### Added

- Initial release
- **Account Utilities** for validation and parsing
- **PDA Helpers** with builder pattern
- **Token Utilities** for SPL token operations
- **Rent Cleaner** for basic rent recovery
- **Jupiter Integration** for DEX swaps (optional feature)
- Examples: rent_cleaner, token_burn, jupiter_swap
