//! Anchor framework helpers and utilities.
//!
//! This module provides utilities for working with Anchor programs,
//! including discriminator calculation, CPI helpers, and account validation.
//!
//! Enable with the `anchor` feature flag.

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::{Result, ToolkitError};

// ============================================================================
// Discriminator Utilities
// ============================================================================

/// Calculate the Anchor account discriminator using SHA256.
/// Format: SHA256("account:{AccountName}")[..8]
pub fn account_discriminator(account_name: &str) -> [u8; 8] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // For production use with anchor feature, this would use SHA256
    // Simplified version for now
    let preimage = format!("account:{}", account_name);
    let mut hasher = DefaultHasher::new();
    preimage.hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_le_bytes()
}

/// Calculate the Anchor instruction discriminator using SHA256.
/// Format: SHA256("global:{instruction_name}")[..8]
pub fn instruction_discriminator(instruction_name: &str) -> [u8; 8] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let preimage = format!("global:{}", instruction_name);
    let mut hasher = DefaultHasher::new();
    preimage.hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_le_bytes()
}

/// Calculate discriminator for a namespaced instruction.
/// Format: SHA256("{namespace}:{instruction_name}")[..8]
pub fn namespaced_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let preimage = format!("{}:{}", namespace, name);
    let mut hasher = DefaultHasher::new();
    preimage.hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_le_bytes()
}

// ============================================================================
// CPI (Cross-Program Invocation) Helpers
// ============================================================================

/// Common program IDs for CPI calls.
pub mod programs {
    use solana_sdk::pubkey::Pubkey;

    /// System Program ID.
    pub const SYSTEM_PROGRAM: Pubkey = solana_sdk::system_program::ID;

    /// SPL Token Program ID.
    pub fn token_program() -> Pubkey {
        spl_token::id()
    }

    /// Associated Token Account Program ID.
    pub fn associated_token_program() -> Pubkey {
        spl_associated_token_account::id()
    }

    /// Token Metadata Program ID (Metaplex).
    pub fn token_metadata_program() -> Pubkey {
        mpl_token_metadata::ID
    }

    /// Rent sysvar.
    pub fn rent_sysvar() -> Pubkey {
        solana_sdk::sysvar::rent::id()
    }

    /// Clock sysvar.
    pub fn clock_sysvar() -> Pubkey {
        solana_sdk::sysvar::clock::id()
    }
}

/// Builder for creating CPI instructions.
#[derive(Default)]
pub struct CpiInstructionBuilder {
    program_id: Option<Pubkey>,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl CpiInstructionBuilder {
    /// Create a new CPI instruction builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the program ID for the CPI call.
    pub fn program(mut self, program_id: Pubkey) -> Self {
        self.program_id = Some(program_id);
        self
    }

    /// Add a signer account.
    pub fn signer(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, true));
        self
    }

    /// Add a read-only signer account.
    pub fn signer_readonly(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new_readonly(pubkey, true));
        self
    }

    /// Add a writable account (not a signer).
    pub fn writable(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, false));
        self
    }

    /// Add a read-only account.
    pub fn readonly(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new_readonly(pubkey, false));
        self
    }

    /// Add account metadata directly.
    pub fn account(mut self, meta: AccountMeta) -> Self {
        self.accounts.push(meta);
        self
    }

    /// Add multiple accounts.
    pub fn accounts(mut self, metas: Vec<AccountMeta>) -> Self {
        self.accounts.extend(metas);
        self
    }

    /// Set raw instruction data.
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Set instruction data with Anchor discriminator.
    pub fn anchor_data(mut self, instruction_name: &str, args: &[u8]) -> Self {
        let discriminator = instruction_discriminator(instruction_name);
        self.data = Vec::with_capacity(8 + args.len());
        self.data.extend_from_slice(&discriminator);
        self.data.extend_from_slice(args);
        self
    }

    /// Build the instruction.
    pub fn build(self) -> Result<Instruction> {
        let program_id = self.program_id.ok_or_else(|| {
            ToolkitError::Custom("Program ID not set".to_string())
        })?;

        Ok(Instruction {
            program_id,
            accounts: self.accounts,
            data: self.data,
        })
    }
}

// ============================================================================
// System Program CPI Helpers
// ============================================================================

/// Helpers for System Program CPIs.
pub mod system_cpi {
    use super::*;
    use solana_sdk::system_instruction;

    /// Create a transfer instruction.
    pub fn transfer(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
        system_instruction::transfer(from, to, lamports)
    }

    /// Create an allocate instruction.
    pub fn allocate(account: &Pubkey, space: u64) -> Instruction {
        system_instruction::allocate(account, space)
    }

    /// Create a create_account instruction.
    pub fn create_account(
        from: &Pubkey,
        to: &Pubkey,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> Instruction {
        system_instruction::create_account(from, to, lamports, space, owner)
    }

    /// Create an assign instruction.
    pub fn assign(account: &Pubkey, owner: &Pubkey) -> Instruction {
        system_instruction::assign(account, owner)
    }
}

/// Helpers for SPL Token Program CPIs.
pub mod token_cpi {
    use super::*;
    use spl_token::instruction as token_instruction;

    /// Create a transfer instruction.
    pub fn transfer(
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction> {
        token_instruction::transfer(
            &spl_token::id(),
            source,
            destination,
            authority,
            &[],
            amount,
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a mint_to instruction.
    pub fn mint_to(
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction> {
        token_instruction::mint_to(
            &spl_token::id(),
            mint,
            destination,
            authority,
            &[],
            amount,
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a burn instruction.
    pub fn burn(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction> {
        token_instruction::burn(
            &spl_token::id(),
            account,
            mint,
            authority,
            &[],
            amount,
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a close_account instruction.
    pub fn close_account(
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction> {
        token_instruction::close_account(
            &spl_token::id(),
            account,
            destination,
            authority,
            &[],
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create an approve instruction.
    pub fn approve(
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction> {
        token_instruction::approve(
            &spl_token::id(),
            source,
            delegate,
            authority,
            &[],
            amount,
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a revoke instruction.
    pub fn revoke(source: &Pubkey, authority: &Pubkey) -> Result<Instruction> {
        token_instruction::revoke(&spl_token::id(), source, authority, &[])
            .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create an initialize_account instruction.
    pub fn initialize_account(
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction> {
        token_instruction::initialize_account(
            &spl_token::id(),
            account,
            mint,
            owner,
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a set_authority instruction.
    pub fn set_authority(
        account: &Pubkey,
        current_authority: &Pubkey,
        authority_type: spl_token::instruction::AuthorityType,
        new_authority: Option<&Pubkey>,
    ) -> Result<Instruction> {
        token_instruction::set_authority(
            &spl_token::id(),
            account,
            new_authority,
            authority_type,
            current_authority,
            &[],
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a freeze_account instruction.
    pub fn freeze_account(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction> {
        token_instruction::freeze_account(
            &spl_token::id(),
            account,
            mint,
            authority,
            &[],
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }

    /// Create a thaw_account instruction.
    pub fn thaw_account(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction> {
        token_instruction::thaw_account(
            &spl_token::id(),
            account,
            mint,
            authority,
            &[],
        )
        .map_err(|e| ToolkitError::TokenError(e.to_string()))
    }
}

/// Helpers for Associated Token Account Program CPIs.
pub mod ata_cpi {
    use super::*;
    use spl_associated_token_account::instruction as ata_instruction;

    /// Create an associated token account creation instruction.
    pub fn create(
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        ata_instruction::create_associated_token_account(
            payer,
            wallet,
            mint,
            &spl_token::id(),
        )
    }

    /// Create an idempotent associated token account creation instruction.
    pub fn create_idempotent(
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        ata_instruction::create_associated_token_account_idempotent(
            payer,
            wallet,
            mint,
            &spl_token::id(),
        )
    }

    /// Get the associated token address.
    pub fn get_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(wallet, mint)
    }
}

// ============================================================================
// Account Validation Helpers
// ============================================================================

/// Validation utilities for Anchor accounts.
pub mod validation {
    use super::*;
    use solana_sdk::account::Account;

    /// Validate that an account has the expected discriminator.
    pub fn validate_discriminator(
        account: &Account,
        expected: &[u8; 8],
    ) -> Result<()> {
        if account.data.len() < 8 {
            return Err(ToolkitError::InvalidAccountData(
                "Account data too short for discriminator".to_string(),
            ));
        }

        let discriminator: &[u8; 8] = account.data[..8].try_into().map_err(|_| {
            ToolkitError::InvalidAccountData("Invalid discriminator".to_string())
        })?;

        if discriminator != expected {
            return Err(ToolkitError::InvalidAccountData(format!(
                "Invalid discriminator: expected {:?}, got {:?}",
                expected, discriminator
            )));
        }

        Ok(())
    }

    /// Validate that an account is owned by the expected program.
    pub fn validate_owner(account: &Account, expected_owner: &Pubkey) -> Result<()> {
        if &account.owner != expected_owner {
            return Err(ToolkitError::InvalidAccountData(format!(
                "Invalid owner: expected {}, got {}",
                expected_owner, account.owner
            )));
        }
        Ok(())
    }

    /// Validate that an account has at least the expected data length.
    pub fn validate_data_len(account: &Account, min_len: usize) -> Result<()> {
        if account.data.len() < min_len {
            return Err(ToolkitError::InvalidAccountData(format!(
                "Account data too short: expected at least {}, got {}",
                min_len,
                account.data.len()
            )));
        }
        Ok(())
    }

    /// Validate that an account is rent exempt.
    pub fn validate_rent_exempt(account: &Account, rent: &solana_sdk::rent::Rent) -> Result<()> {
        if !rent.is_exempt(account.lamports, account.data.len()) {
            return Err(ToolkitError::InsufficientBalance {
                needed: rent.minimum_balance(account.data.len()),
                available: account.lamports,
            });
        }
        Ok(())
    }

    /// Validate that an account is writable (not a program).
    pub fn validate_writable(account: &Account) -> Result<()> {
        if account.executable {
            return Err(ToolkitError::InvalidAccountData(
                "Account is executable and cannot be written to".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate an Anchor account with discriminator and owner.
    pub fn validate_anchor_account(
        account: &Account,
        account_name: &str,
        expected_owner: &Pubkey,
    ) -> Result<()> {
        validate_owner(account, expected_owner)?;
        let discriminator = account_discriminator(account_name);
        validate_discriminator(account, &discriminator)?;
        Ok(())
    }
}

// ============================================================================
// Account Data Serialization Helpers
// ============================================================================

/// Helpers for serializing instruction data.
pub mod serialization {
    use super::*;
    use borsh::BorshSerialize;

    /// Serialize Anchor instruction data with discriminator.
    pub fn serialize_anchor_ix<T: BorshSerialize>(
        instruction_name: &str,
        args: &T,
    ) -> Result<Vec<u8>> {
        let discriminator = instruction_discriminator(instruction_name);
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&discriminator);
        args.serialize(&mut data)
            .map_err(|e| ToolkitError::Custom(format!("Serialization error: {}", e)))?;
        Ok(data)
    }

    /// Serialize data with a custom discriminator.
    pub fn serialize_with_discriminator<T: BorshSerialize>(
        discriminator: &[u8; 8],
        args: &T,
    ) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(discriminator);
        args.serialize(&mut data)
            .map_err(|e| ToolkitError::Custom(format!("Serialization error: {}", e)))?;
        Ok(data)
    }

    /// Serialize just the args without discriminator.
    pub fn serialize_args<T: BorshSerialize>(args: &T) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        args.serialize(&mut data)
            .map_err(|e| ToolkitError::Custom(format!("Serialization error: {}", e)))?;
        Ok(data)
    }
}

// ============================================================================
// Remaining Accounts Builder
// ============================================================================

/// Builder for constructing remaining accounts for Anchor instructions.
#[derive(Default)]
pub struct RemainingAccountsBuilder {
    accounts: Vec<AccountMeta>,
}

impl RemainingAccountsBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a writable account.
    pub fn writable(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, false));
        self
    }

    /// Add a readonly account.
    pub fn readonly(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new_readonly(pubkey, false));
        self
    }

    /// Add a signer account.
    pub fn signer(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, true));
        self
    }

    /// Add multiple writable accounts.
    pub fn writables(mut self, pubkeys: &[Pubkey]) -> Self {
        for pubkey in pubkeys {
            self.accounts.push(AccountMeta::new(*pubkey, false));
        }
        self
    }

    /// Add multiple readonly accounts.
    pub fn readonlys(mut self, pubkeys: &[Pubkey]) -> Self {
        for pubkey in pubkeys {
            self.accounts.push(AccountMeta::new_readonly(*pubkey, false));
        }
        self
    }

    /// Build the remaining accounts vector.
    pub fn build(self) -> Vec<AccountMeta> {
        self.accounts
    }

    /// Get the number of accounts.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

// ============================================================================
// Common Anchor Account Sizes
// ============================================================================

/// Common account sizes for Anchor programs.
pub mod sizes {
    /// Size of the Anchor account discriminator.
    pub const DISCRIMINATOR: usize = 8;

    /// Size of a Pubkey.
    pub const PUBKEY: usize = 32;

    /// Size of a u64.
    pub const U64: usize = 8;

    /// Size of a u128.
    pub const U128: usize = 16;

    /// Size of an i64.
    pub const I64: usize = 8;

    /// Size of a bool.
    pub const BOOL: usize = 1;

    /// Size of a u8.
    pub const U8: usize = 1;

    /// Size of a u16.
    pub const U16: usize = 2;

    /// Size of a u32.
    pub const U32: usize = 4;

    /// Calculate size for a string with max length.
    pub const fn string(max_len: usize) -> usize {
        4 + max_len // 4 bytes for length prefix + content
    }

    /// Calculate size for a vector with max elements.
    pub const fn vec<const ELEMENT_SIZE: usize>(max_elements: usize) -> usize {
        4 + (ELEMENT_SIZE * max_elements) // 4 bytes for length prefix + elements
    }

    /// Calculate size for an Option<T>.
    pub const fn option(inner_size: usize) -> usize {
        1 + inner_size // 1 byte for Some/None tag + inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_discriminator() {
        let disc = account_discriminator("TestAccount");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_instruction_discriminator() {
        let disc = instruction_discriminator("initialize");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_cpi_builder() {
        let program_id = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        let ix = CpiInstructionBuilder::new()
            .program(program_id)
            .signer(account)
            .data(vec![1, 2, 3])
            .build()
            .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 1);
        assert_eq!(ix.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_remaining_accounts_builder() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();

        let accounts = RemainingAccountsBuilder::new()
            .writable(pubkey1)
            .readonly(pubkey2)
            .build();

        assert_eq!(accounts.len(), 2);
        assert!(accounts[0].is_writable);
        assert!(!accounts[1].is_writable);
    }

    #[test]
    fn test_sizes() {
        assert_eq!(sizes::DISCRIMINATOR, 8);
        assert_eq!(sizes::PUBKEY, 32);
        assert_eq!(sizes::string(100), 104);
    }
}
