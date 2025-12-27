//! Amount validation utilities.
//!
//! Validates transaction amounts to prevent common mistakes like sending
//! entire balances accidentally or adding too many zeros.

use crate::{Result, ToolkitError};

/// Lamports per SOL constant.
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Result of amount validation.
#[derive(Debug, Clone)]
pub struct AmountValidation {
    /// Whether the amount is valid.
    pub is_valid: bool,
    /// Warnings about the amount (non-blocking).
    pub warnings: Vec<String>,
    /// Whether this amount requires explicit user confirmation.
    pub requires_confirmation: bool,
    /// Human-readable representation of the amount.
    pub human_readable: String,
    /// The validated amount in smallest units.
    pub amount: u64,
}

/// Warning about a potential amount issue.
#[derive(Debug, Clone)]
pub struct AmountWarning {
    /// Warning message.
    pub message: String,
    /// Severity level (low, medium, high).
    pub severity: WarningSeverity,
}

/// Severity of an amount warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

/// Potential magnitude error detection result.
#[derive(Debug, Clone)]
pub struct MagnitudeCheck {
    /// Whether a magnitude error was detected.
    pub likely_error: bool,
    /// The intended amount (what user probably meant).
    pub intended_amount: f64,
    /// The actual amount that would be sent.
    pub actual_amount: f64,
    /// Explanation of the potential error.
    pub explanation: String,
}

/// Amount validator for transaction safety.
pub struct AmountValidator;

impl AmountValidator {
    /// Validate a token amount for safety issues.
    ///
    /// # Arguments
    /// * `amount` - Amount in smallest units (lamports for SOL)
    /// * `decimals` - Token decimals (9 for SOL)
    /// * `balance` - Current balance in smallest units
    ///
    /// # Returns
    /// Validation result with any warnings
    pub fn validate_amount(
        amount: u64,
        decimals: u8,
        balance: u64,
    ) -> AmountValidation {
        let mut warnings = Vec::new();
        let mut requires_confirmation = false;

        // Check if sending entire or nearly entire balance
        if balance > 0 {
            let percentage = (amount as f64 / balance as f64) * 100.0;

            if percentage > 99.0 {
                warnings.push(
                    "Sending entire balance. No funds will remain for fees.".to_string()
                );
                requires_confirmation = true;
            } else if percentage > 90.0 {
                warnings.push(format!(
                    "Sending {:.1}% of balance. Only {:.6} will remain.",
                    percentage,
                    Self::format_amount(balance - amount, decimals)
                ));
                requires_confirmation = true;
            }
        }

        // Check for zero amount
        if amount == 0 {
            warnings.push("Amount is zero.".to_string());
        }

        // Check if amount exceeds balance
        if amount > balance {
            warnings.push(format!(
                "Amount ({}) exceeds balance ({}).",
                Self::format_amount(amount, decimals),
                Self::format_amount(balance, decimals)
            ));
        }

        let human_readable = Self::format_amount(amount, decimals);

        AmountValidation {
            is_valid: amount <= balance && amount > 0,
            warnings,
            requires_confirmation,
            human_readable,
            amount,
        }
    }

    /// Convert a human-readable amount to token units safely.
    ///
    /// # Arguments
    /// * `human_amount` - Amount in human units (e.g., 1.5 for 1.5 SOL)
    /// * `decimals` - Token decimals (9 for SOL)
    ///
    /// # Returns
    /// Amount in smallest units, or error if conversion fails
    ///
    /// # Example
    /// ```ignore
    /// let lamports = AmountValidator::human_to_token_amount(1.5, 9)?;
    /// assert_eq!(lamports, 1_500_000_000);
    /// ```
    pub fn human_to_token_amount(human_amount: f64, decimals: u8) -> Result<u64> {
        if human_amount < 0.0 {
            return Err(ToolkitError::AmountValidation {
                message: "Amount cannot be negative".to_string(),
            });
        }

        if human_amount.is_nan() || human_amount.is_infinite() {
            return Err(ToolkitError::AmountValidation {
                message: "Amount must be a valid number".to_string(),
            });
        }

        let multiplier = 10u64.pow(decimals as u32);
        let token_amount = human_amount * (multiplier as f64);

        // Check for overflow
        if token_amount > u64::MAX as f64 {
            return Err(ToolkitError::AmountValidation {
                message: "Amount too large, would overflow".to_string(),
            });
        }

        // Round to avoid floating point precision issues
        let result = token_amount.round() as u64;

        Ok(result)
    }

    /// Convert token units to human-readable amount.
    ///
    /// # Arguments
    /// * `token_amount` - Amount in smallest units
    /// * `decimals` - Token decimals
    ///
    /// # Returns
    /// Human-readable amount as f64
    pub fn token_to_human_amount(token_amount: u64, decimals: u8) -> f64 {
        let divisor = 10u64.pow(decimals as u32);
        token_amount as f64 / divisor as f64
    }

    /// Format an amount for display.
    ///
    /// # Arguments
    /// * `amount` - Amount in smallest units
    /// * `decimals` - Token decimals
    ///
    /// # Returns
    /// Formatted string (e.g., "1.500000000" for 1.5 SOL)
    pub fn format_amount(amount: u64, decimals: u8) -> String {
        let human = Self::token_to_human_amount(amount, decimals);
        format!("{:.width$}", human, width = decimals as usize)
    }

    /// Format an amount with symbol for display.
    ///
    /// # Arguments
    /// * `amount` - Amount in smallest units
    /// * `decimals` - Token decimals
    /// * `symbol` - Token symbol (e.g., "SOL")
    ///
    /// # Returns
    /// Formatted string (e.g., "1.5 SOL")
    pub fn format_amount_with_symbol(amount: u64, decimals: u8, symbol: &str) -> String {
        let human = Self::token_to_human_amount(amount, decimals);
        // Trim trailing zeros for cleaner display
        let formatted = format!("{:.width$}", human, width = decimals as usize);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("{} {}", trimmed, symbol)
    }

    /// Check if an amount requires explicit user confirmation.
    ///
    /// # Arguments
    /// * `amount_usd` - Estimated USD value of the amount
    /// * `threshold` - USD threshold for requiring confirmation (default: 1000)
    ///
    /// # Returns
    /// true if amount exceeds threshold
    pub fn requires_confirmation(amount_usd: f64, threshold: f64) -> bool {
        amount_usd >= threshold
    }

    /// Detect potential magnitude errors (wrong number of zeros).
    ///
    /// Compares what the user typed vs what would actually be sent,
    /// detecting cases like typing "1000" when meaning "1.000".
    ///
    /// # Arguments
    /// * `input_string` - What the user typed (e.g., "1000")
    /// * `actual_amount` - The computed amount in human units
    /// * `decimals` - Token decimals
    ///
    /// # Returns
    /// MagnitudeCheck with details about potential errors
    pub fn detect_magnitude_error(
        input_string: &str,
        actual_amount: f64,
        _decimals: u8,
    ) -> MagnitudeCheck {
        // Parse the input as a number
        let parsed: f64 = match input_string.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                return MagnitudeCheck {
                    likely_error: false,
                    intended_amount: 0.0,
                    actual_amount,
                    explanation: "Could not parse input".to_string(),
                }
            }
        };

        // Check for common magnitude errors
        // Case 1: User typed "1000" but meant "1.000" (European decimal notation)
        // Case 2: User typed "1000" but meant "1" (extra zeros)
        // Case 3: User typed "0.001" but meant "0.1" (missed zeros)

        let mut likely_error = false;
        let mut intended_amount = parsed;
        let mut explanation = String::new();

        // Check if the amount seems unusually large (>100x typical)
        // This is a heuristic based on common token values
        if parsed >= 1000.0 {
            // Could be European notation: 1.000 written as 1000
            let european_interpretation = parsed / 1000.0;

            // If the European interpretation makes more sense (small reasonable amount)
            if european_interpretation >= 0.001 && european_interpretation <= 100.0 {
                likely_error = true;
                intended_amount = european_interpretation;
                explanation = format!(
                    "Did you mean {:.3} instead of {}? (European decimal notation)",
                    european_interpretation, parsed
                );
            }
        }

        // Check for suspicious round numbers that might indicate extra zeros
        if parsed >= 100.0 && parsed == parsed.floor() {
            let zeros = (parsed.log10().floor() as u32).saturating_sub(1);
            if zeros >= 2 {
                let possible_intended = parsed / 10f64.powi(zeros as i32);
                if possible_intended >= 0.1 && possible_intended <= 10.0 {
                    if !likely_error {
                        likely_error = true;
                        intended_amount = possible_intended;
                        explanation = format!(
                            "Large round number detected. Did you mean {:.1} instead of {}?",
                            possible_intended, parsed
                        );
                    }
                }
            }
        }

        MagnitudeCheck {
            likely_error,
            intended_amount,
            actual_amount,
            explanation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_amount_passes() {
        let balance = 10 * LAMPORTS_PER_SOL; // 10 SOL
        let amount = 1 * LAMPORTS_PER_SOL;   // 1 SOL

        let result = AmountValidator::validate_amount(amount, 9, balance);

        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
        assert!(!result.requires_confirmation);
    }

    #[test]
    fn test_decimal_conversion_accurate() {
        // 1.5 SOL should equal 1_500_000_000 lamports
        let result = AmountValidator::human_to_token_amount(1.5, 9);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1_500_000_000);

        // 0.001 SOL = 1_000_000 lamports
        let result = AmountValidator::human_to_token_amount(0.001, 9);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1_000_000);

        // 100 SOL
        let result = AmountValidator::human_to_token_amount(100.0, 9);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100_000_000_000);

        // Test with 6 decimals (USDC)
        let result = AmountValidator::human_to_token_amount(1.5, 6);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1_500_000);
    }

    #[test]
    fn test_warns_sending_full_balance() {
        let balance = 10 * LAMPORTS_PER_SOL;

        // Sending 95% should trigger warning
        let amount = (balance as f64 * 0.95) as u64;
        let result = AmountValidator::validate_amount(amount, 9, balance);
        assert!(result.requires_confirmation);
        assert!(!result.warnings.is_empty());

        // Sending 50% should NOT trigger warning
        let amount = balance / 2;
        let result = AmountValidator::validate_amount(amount, 9, balance);
        assert!(!result.requires_confirmation);
        assert!(result.warnings.is_empty());

        // Sending 100% should definitely trigger warning
        let result = AmountValidator::validate_amount(balance, 9, balance);
        assert!(result.requires_confirmation);
        assert!(result.warnings.iter().any(|w| w.contains("entire balance")));
    }

    #[test]
    fn test_catches_magnitude_error() {
        // User types "1000" when they probably meant "1.000" or "1"
        let check = AmountValidator::detect_magnitude_error("1000", 1000.0, 9);
        assert!(check.likely_error);
        assert!(check.intended_amount < 1000.0);

        // User types "10000" - likely meant "10" or "1.0000"
        let check = AmountValidator::detect_magnitude_error("10000", 10000.0, 9);
        assert!(check.likely_error);

        // Normal amount "1.5" should not trigger
        let check = AmountValidator::detect_magnitude_error("1.5", 1.5, 9);
        assert!(!check.likely_error);

        // Small amount "0.5" should not trigger
        let check = AmountValidator::detect_magnitude_error("0.5", 0.5, 9);
        assert!(!check.likely_error);
    }

    #[test]
    fn test_requires_confirmation_large_amounts() {
        // $500 should not require confirmation with $1000 threshold
        assert!(!AmountValidator::requires_confirmation(500.0, 1000.0));

        // $1000 should require confirmation
        assert!(AmountValidator::requires_confirmation(1000.0, 1000.0));

        // $5000 should definitely require confirmation
        assert!(AmountValidator::requires_confirmation(5000.0, 1000.0));

        // Custom threshold of $100
        assert!(AmountValidator::requires_confirmation(100.0, 100.0));
        assert!(!AmountValidator::requires_confirmation(99.99, 100.0));
    }

    #[test]
    fn test_human_readable_formatting() {
        // 1.5 SOL
        let formatted = AmountValidator::format_amount_with_symbol(
            1_500_000_000,
            9,
            "SOL"
        );
        assert_eq!(formatted, "1.5 SOL");

        // 0.001 SOL
        let formatted = AmountValidator::format_amount_with_symbol(
            1_000_000,
            9,
            "SOL"
        );
        assert_eq!(formatted, "0.001 SOL");

        // 100 USDC (6 decimals)
        let formatted = AmountValidator::format_amount_with_symbol(
            100_000_000,
            6,
            "USDC"
        );
        assert_eq!(formatted, "100 USDC");

        // 1 lamport
        let formatted = AmountValidator::format_amount_with_symbol(
            1,
            9,
            "SOL"
        );
        assert_eq!(formatted, "0.000000001 SOL");
    }

    #[test]
    fn test_negative_amount_rejected() {
        let result = AmountValidator::human_to_token_amount(-1.0, 9);
        assert!(result.is_err());
    }

    #[test]
    fn test_nan_amount_rejected() {
        let result = AmountValidator::human_to_token_amount(f64::NAN, 9);
        assert!(result.is_err());
    }

    #[test]
    fn test_amount_exceeds_balance() {
        let balance = 1 * LAMPORTS_PER_SOL;
        let amount = 2 * LAMPORTS_PER_SOL;

        let result = AmountValidator::validate_amount(amount, 9, balance);
        assert!(!result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("exceeds balance")));
    }

    #[test]
    fn test_zero_amount_warning() {
        let balance = 1 * LAMPORTS_PER_SOL;

        let result = AmountValidator::validate_amount(0, 9, balance);
        assert!(!result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("zero")));
    }
}
