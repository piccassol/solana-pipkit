//! Address verification example.
//!
//! Demonstrates how to verify Solana addresses before sending transactions,
//! catching typos and invalid addresses before they cause fund losses.
//!
//! Run with: cargo run --example address_verification

use solana_pipkit::safety::{AddressVerifier, AddressComparison};

fn main() {
    println!("=== Solana Address Verification Demo ===\n");

    // Example 1: Verify a valid address
    println!("1. Verifying a valid address:");
    let valid_address = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";

    match AddressVerifier::verify_address(valid_address) {
        Ok(pubkey) => {
            let short = AddressVerifier::format_address_short(&pubkey);
            println!("   Valid: {} ({})", short, pubkey);
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    // Example 2: Invalid base58 character
    println!("\n2. Detecting invalid base58 characters:");
    let invalid_base58 = "0xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";

    match AddressVerifier::verify_address(invalid_base58) {
        Ok(_) => println!("   Passed (unexpected)"),
        Err(e) => println!("   Caught error: {}", e),
    }

    // Example 3: Wrong length
    println!("\n3. Detecting wrong length:");
    let too_short = "7xKXtg2CW87d97";

    match AddressVerifier::verify_address(too_short) {
        Ok(_) => println!("   Passed (unexpected)"),
        Err(e) => println!("   Caught error: {}", e),
    }

    // Example 4: Detecting typos
    println!("\n4. Detecting typos between addresses:");
    let intended = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
    let typo = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsV";

    let comparison = AddressVerifier::compare_addresses(intended, typo);

    if comparison.matches {
        println!("   Addresses match");
    } else {
        println!("   Addresses differ:");
        println!("      Difference count: {}", comparison.difference_count);
        println!("      Positions: {:?}", comparison.difference_positions);
        if comparison.likely_typo {
            println!("      WARNING: This looks like a typo!");
        }
    }

    // Example 5: User confirmation flow
    println!("\n5. Confirmation flow before sending:");
    let recipient = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";

    match AddressVerifier::verify_full(recipient) {
        Ok(verification) => {
            println!("   About to send to: {}", verification.short_display);
            println!("   Full address: {}", verification.pubkey);
            println!("   ");
            println!("   Please verify the first 4 and last 4 characters match");
            println!("   what you expect before confirming the transaction.");
        }
        Err(e) => {
            println!("   Cannot proceed: {}", e);
        }
    }

    // Example 6: Whitespace handling
    println!("\n6. Handling whitespace in pasted addresses:");
    let with_whitespace = "  7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU  ";

    match AddressVerifier::verify_address(with_whitespace) {
        Ok(pubkey) => {
            println!("   Whitespace trimmed, address valid: {}",
                     AddressVerifier::format_address_short(&pubkey));
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    println!("\n=== Demo Complete ===");
    println!("\nThis safety check prevents the #1 cause of crypto losses:");
    println!("sending funds to wrong addresses due to typos or copy/paste errors.");
}
