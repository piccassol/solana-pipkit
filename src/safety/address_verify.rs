//! Address verification utilities.
//!
//! Validates Solana addresses to prevent typo-related losses.

use crate::{Result, ToolkitError};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Address verification result with detailed information.
#[derive(Debug, Clone)]
pub struct AddressVerification {
    /// The verified pubkey.
    pub pubkey: Pubkey,
    /// Whether the address is valid.
    pub is_valid: bool,
    /// Shortened display format (e.g., "7xKX...8AsU").
    pub short_display: String,
}

/// Address verifier for validating Solana addresses.
pub struct AddressVerifier;

impl AddressVerifier {
    /// Verify a Solana address string and return the parsed Pubkey.
    ///
    /// # Arguments
    /// * `address` - Base58 encoded Solana address string
    ///
    /// # Returns
    /// * `Ok(Pubkey)` - Valid address parsed as Pubkey
    /// * `Err(ToolkitError)` - Detailed error explaining why validation failed
    ///
    /// # Example
    /// ```ignore
    /// let pubkey = AddressVerifier::verify_address("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU")?;
    /// ```
    pub fn verify_address(address: &str) -> Result<Pubkey> {
        // Check for empty input
        if address.is_empty() {
            return Err(ToolkitError::InvalidAddress {
                address: address.to_string(),
                reason: "Address cannot be empty".to_string(),
            });
        }

        // Trim whitespace
        let address = address.trim();

        // Check length (Solana addresses are 32-44 base58 characters)
        if address.len() < 32 || address.len() > 44 {
            return Err(ToolkitError::InvalidAddress {
                address: address.to_string(),
                reason: format!(
                    "Invalid length: expected 32-44 characters, got {}",
                    address.len()
                ),
            });
        }

        // Check for invalid base58 characters
        // Base58 excludes: 0, O, I, l
        let invalid_chars: Vec<char> = address
            .chars()
            .filter(|c| !is_valid_base58_char(*c))
            .collect();

        if !invalid_chars.is_empty() {
            return Err(ToolkitError::InvalidAddress {
                address: address.to_string(),
                reason: format!(
                    "Invalid base58 characters: {:?}. Base58 does not include 0, O, I, or l",
                    invalid_chars
                ),
            });
        }

        // Parse as Pubkey
        Pubkey::from_str(address).map_err(|e| ToolkitError::InvalidAddress {
            address: address.to_string(),
            reason: format!("Failed to parse address: {}", e),
        })
    }

    /// Verify address and return full verification result.
    pub fn verify_full(address: &str) -> Result<AddressVerification> {
        let pubkey = Self::verify_address(address)?;
        Ok(AddressVerification {
            pubkey,
            is_valid: true,
            short_display: Self::format_address_short(&pubkey),
        })
    }

    /// Format an address in shortened form for user confirmation.
    ///
    /// Returns format like "7xKX...8AsU" which is easy for humans to verify.
    pub fn format_address_short(pubkey: &Pubkey) -> String {
        let full = pubkey.to_string();
        if full.len() <= 8 {
            return full;
        }
        format!("{}...{}", &full[..4], &full[full.len() - 4..])
    }

    /// Compare two addresses for equality, with typo detection.
    ///
    /// Returns detailed information about differences if addresses don't match.
    pub fn compare_addresses(addr1: &str, addr2: &str) -> AddressComparison {
        let trimmed1 = addr1.trim();
        let trimmed2 = addr2.trim();

        if trimmed1 == trimmed2 {
            return AddressComparison {
                matches: true,
                difference_count: 0,
                difference_positions: vec![],
                likely_typo: false,
            };
        }

        // Count character differences
        let chars1: Vec<char> = trimmed1.chars().collect();
        let chars2: Vec<char> = trimmed2.chars().collect();

        let mut differences = Vec::new();

        // Compare character by character
        let max_len = chars1.len().max(chars2.len());
        for i in 0..max_len {
            let c1 = chars1.get(i);
            let c2 = chars2.get(i);

            if c1 != c2 {
                differences.push(i);
            }
        }

        let difference_count = differences.len();
        // Likely a typo if only 1-2 characters differ
        let likely_typo = difference_count > 0 && difference_count <= 2;

        AddressComparison {
            matches: false,
            difference_count,
            difference_positions: differences,
            likely_typo,
        }
    }
}

/// Result of comparing two addresses.
#[derive(Debug, Clone)]
pub struct AddressComparison {
    /// Whether the addresses match exactly.
    pub matches: bool,
    /// Number of character differences.
    pub difference_count: usize,
    /// Positions where characters differ.
    pub difference_positions: Vec<usize>,
    /// Whether this looks like a typo (1-2 char differences).
    pub likely_typo: bool,
}

/// Check if a character is valid in base58 encoding.
fn is_valid_base58_char(c: char) -> bool {
    matches!(c,
        '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_ADDRESS: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";

    #[test]
    fn test_valid_address_passes() {
        let result = AddressVerifier::verify_address(VALID_ADDRESS);
        assert!(result.is_ok());
        let pubkey = result.unwrap();
        assert_eq!(pubkey.to_string(), VALID_ADDRESS);
    }

    #[test]
    fn test_invalid_base58_rejected() {
        // Contains '0' which is invalid in base58
        let invalid = "0xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
        let result = AddressVerifier::verify_address(invalid);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Invalid base58"));
    }

    #[test]
    fn test_wrong_length_rejected() {
        // Too short
        let short = "7xKXtg2CW87d97";
        let result = AddressVerifier::verify_address(short);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Invalid length"));
    }

    #[test]
    fn test_catches_typo() {
        // Last character changed from U to V
        let typo = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsV";

        let comparison = AddressVerifier::compare_addresses(VALID_ADDRESS, typo);

        assert!(!comparison.matches);
        assert_eq!(comparison.difference_count, 1);
        assert!(comparison.likely_typo);
    }

    #[test]
    fn test_format_address_short() {
        let pubkey = Pubkey::from_str(VALID_ADDRESS).unwrap();
        let short = AddressVerifier::format_address_short(&pubkey);

        assert_eq!(short, "7xKX...gAsU");
        assert_eq!(short.len(), 11); // 4 + 3 + 4
    }

    #[test]
    fn test_empty_address_rejected() {
        let result = AddressVerifier::verify_address("");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("empty"));
    }

    #[test]
    fn test_whitespace_trimmed() {
        let with_whitespace = format!("  {}  ", VALID_ADDRESS);
        let result = AddressVerifier::verify_address(&with_whitespace);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_full_returns_complete_info() {
        let result = AddressVerifier::verify_full(VALID_ADDRESS);
        assert!(result.is_ok());

        let verification = result.unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.short_display, "7xKX...gAsU");
    }
}
