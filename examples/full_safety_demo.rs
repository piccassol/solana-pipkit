//! Full Safety Protocol Demo
//!
//! Demonstrates all safety features for transaction validation.
//!
//! Run with: cargo run --example full_safety_demo

use solana_pipkit::prelude::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    println!("=== Solana Safety Protocol Demo ===\n");

    // Test addresses
    let sender = Pubkey::from_str("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU").unwrap();
    let recipient = Pubkey::from_str("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM").unwrap();

    // Simulated balance: 10 SOL
    let balance = 10 * LAMPORTS_PER_SOL;

    // Create safety protocol with price info
    let protocol = SafetyProtocol::new()
        .token_price(100.0) // $100 per SOL
        .large_amount_threshold(1000.0); // Confirm transfers > $1000

    // --- Test 1: Valid small transfer ---
    println!("Test 1: Valid small transfer (1 SOL)");
    let amount = 1 * LAMPORTS_PER_SOL;
    let report = protocol.validate_offline(&sender, &recipient, amount, 9, balance);
    print_report(&report);

    // --- Test 2: Large transfer requiring confirmation ---
    println!("\nTest 2: Large transfer (15 SOL = $1500)");
    let amount = 15 * LAMPORTS_PER_SOL;
    let report = protocol.validate_offline(&sender, &recipient, amount, 9, 100 * LAMPORTS_PER_SOL);
    print_report(&report);

    // --- Test 3: Full balance warning ---
    println!("\nTest 3: Sending 95% of balance");
    let amount = (balance as f64 * 0.95) as u64;
    let report = protocol.validate_offline(&sender, &recipient, amount, 9, balance);
    print_report(&report);

    // --- Test 4: Insufficient balance (blocked) ---
    println!("\nTest 4: Insufficient balance (20 SOL from 10 SOL balance)");
    let amount = 20 * LAMPORTS_PER_SOL;
    let report = protocol.validate_offline(&sender, &recipient, amount, 9, balance);
    print_report(&report);

    // --- Test 5: Self-transfer warning ---
    println!("\nTest 5: Self-transfer");
    let amount = 1 * LAMPORTS_PER_SOL;
    let report = protocol.validate_offline(&sender, &sender, amount, 9, balance);
    print_report(&report);

    // --- Test 6: Strict mode ---
    println!("\nTest 6: Strict mode (self-transfer becomes blocker)");
    let strict_protocol = SafetyProtocol::new().strict();
    let report = strict_protocol.validate_offline(&sender, &sender, amount, 9, balance);
    print_report(&report);

    // --- Address Verification ---
    println!("\n=== Address Verification ===\n");

    // Valid address
    println!("Verifying valid address:");
    match AddressVerifier::verify_address("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU") {
        Ok(pubkey) => println!(
            "  Valid: {} (short: {})",
            pubkey,
            AddressVerifier::format_address_short(&pubkey)
        ),
        Err(e) => println!("  Error: {}", e),
    }

    // Invalid address (contains '0')
    println!("\nVerifying invalid address (contains '0'):");
    match AddressVerifier::verify_address("0xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU") {
        Ok(_) => println!("  Unexpectedly valid"),
        Err(e) => println!("  Rejected: {}", e),
    }

    // Typo detection
    println!("\nComparing addresses for typos:");
    let original = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
    let typo = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsV"; // Last char changed
    let comparison = AddressVerifier::compare_addresses(original, typo);
    println!(
        "  Matches: {}, Differences: {}, Likely typo: {}",
        comparison.matches, comparison.difference_count, comparison.likely_typo
    );

    // --- Amount Validation ---
    println!("\n=== Amount Validation ===\n");

    // Decimal conversion
    println!("Decimal conversion:");
    let lamports = AmountValidator::human_to_token_amount(1.5, 9).unwrap();
    println!("  1.5 SOL = {} lamports", lamports);

    let human = AmountValidator::token_to_human_amount(1_500_000_000, 9);
    println!("  1,500,000,000 lamports = {} SOL", human);

    // Formatting
    println!("\nAmount formatting:");
    println!(
        "  {}",
        AmountValidator::format_amount_with_symbol(1_500_000_000, 9, "SOL")
    );
    println!(
        "  {}",
        AmountValidator::format_amount_with_symbol(1_000_000, 6, "USDC")
    );

    // Magnitude error detection
    println!("\nMagnitude error detection:");
    let check = AmountValidator::detect_magnitude_error("1000", 1000.0, 9);
    if check.likely_error {
        println!("  Warning: {}", check.explanation);
    }

    println!("\n=== Demo Complete ===");
}

fn print_report(report: &SafetyReport) {
    println!("  Status: {}", if report.approved { "APPROVED" } else { "BLOCKED" });
    println!("  Risk Level: {}", report.risk_level);
    println!("  From: {} -> To: {}", report.from_display, report.to_display);
    println!("  Amount: {}", report.amount_display);

    if !report.warnings.is_empty() {
        println!("  Warnings:");
        for w in &report.warnings {
            println!("    - {}", w);
        }
    }

    if !report.blockers.is_empty() {
        println!("  Blockers:");
        for b in &report.blockers {
            println!("    - {}", b);
        }
    }

    if report.requires_confirmation {
        println!("  ** Requires user confirmation **");
    }
}
