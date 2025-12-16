//! PDA (Program Derived Address) helpers.
//!
//! Simplified derivation for common Solana PDAs including
//! token metadata, associated token accounts, and custom PDAs.

use solana_sdk::pubkey::Pubkey;

// Common program IDs
pub const TOKEN_METADATA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// Derive a PDA from seeds and program ID.
///
/// # Example
/// ```ignore
/// let (pda, bump) = derive_pda(&[b"vault", user.as_ref()], &program_id);
/// ```
pub fn derive_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Derive PDA with a known bump seed.
pub fn derive_pda_with_bump(
    seeds: &[&[u8]],
    program_id: &Pubkey,
    bump: u8,
) -> Option<Pubkey> {
    let mut seeds_with_bump: Vec<&[u8]> = seeds.to_vec();
    let bump_slice = &[bump];
    seeds_with_bump.push(bump_slice);
    
    Pubkey::create_program_address(&seeds_with_bump, program_id).ok()
}

/// Find the metadata PDA for a given mint.
pub fn find_metadata_pda(mint: &Pubkey) -> (Pubkey, u8) {
    derive_pda(
        &[b"metadata", TOKEN_METADATA_PROGRAM_ID.as_ref(), mint.as_ref()],
        &TOKEN_METADATA_PROGRAM_ID,
    )
}

/// Find the master edition PDA for a given mint.
pub fn find_master_edition_pda(mint: &Pubkey) -> (Pubkey, u8) {
    derive_pda(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint.as_ref(),
            b"edition",
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    )
}

/// Find the associated token account for a wallet and mint.
pub fn find_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    derive_pda(
        &[
            wallet.as_ref(),
            spl_token::id().as_ref(),
            mint.as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
}

/// Find edition marker PDA.
pub fn find_edition_marker_pda(mint: &Pubkey, edition_number: u64) -> (Pubkey, u8) {
    let edition_number_string = (edition_number / 248).to_string();
    derive_pda(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint.as_ref(),
            b"edition",
            edition_number_string.as_bytes(),
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    )
}

/// Validate that a PDA is derived correctly.
pub fn validate_pda(
    pda: &Pubkey,
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> bool {
    let (derived, _) = derive_pda(seeds, program_id);
    derived == *pda
}

/// PDA builder for complex seed patterns.
#[derive(Default)]
pub struct PdaBuilder {
    seeds: Vec<Vec<u8>>,
}

impl PdaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_seed(mut self, seed: &[u8]) -> Self {
        self.seeds.push(seed.to_vec());
        self
    }

    pub fn add_pubkey(mut self, pubkey: &Pubkey) -> Self {
        self.seeds.push(pubkey.to_bytes().to_vec());
        self
    }

    pub fn add_string(mut self, s: &str) -> Self {
        self.seeds.push(s.as_bytes().to_vec());
        self
    }

    pub fn add_u64(mut self, n: u64) -> Self {
        self.seeds.push(n.to_le_bytes().to_vec());
        self
    }

    pub fn derive(self, program_id: &Pubkey) -> (Pubkey, u8) {
        let seed_refs: Vec<&[u8]> = self.seeds.iter().map(|s| s.as_slice()).collect();
        derive_pda(&seed_refs, program_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pda_builder() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let (pda1, bump1) = PdaBuilder::new()
            .add_seed(b"vault")
            .add_pubkey(&user)
            .derive(&program_id);

        let (pda2, bump2) = derive_pda(&[b"vault", user.as_ref()], &program_id);

        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn test_validate_pda() {
        let program_id = Pubkey::new_unique();
        let seeds: &[&[u8]] = &[b"test"];
        let (pda, _) = derive_pda(seeds, &program_id);

        assert!(validate_pda(&pda, seeds, &program_id));
        assert!(!validate_pda(&Pubkey::new_unique(), seeds, &program_id));
    }
}
