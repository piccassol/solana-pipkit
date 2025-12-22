//! Example: Transaction Batching
//!
//! Demonstrates how to use the transaction batching utilities
//! to efficiently execute multiple transactions.

use solana_pipkit::prelude::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Use devnet for testing
    let rpc_url = "https://api.devnet.solana.com";

    // In a real application, load your keypair from a file
    let payer = Keypair::new();
    println!("Payer: {}", payer.pubkey());

    // Example 1: Building a single transaction with the builder
    println!("\n=== Transaction Builder Example ===");

    let recipient = Pubkey::new_unique();
    let transfer_ix = system_instruction::transfer(&payer.pubkey(), &recipient, 1000);

    let builder = TransactionBuilder::new()
        .add_instruction(transfer_ix.clone())
        .compute_units(100_000)
        .priority_fee(1000);

    println!("Instructions in builder: {}", builder.instruction_count());
    println!("Estimated accounts: {}", builder.estimate_accounts());

    // Check if we're close to limits
    if builder.might_exceed_limits() {
        println!("Warning: Transaction might exceed limits");
    }

    // Example 2: Using different transaction configs
    println!("\n=== Transaction Configs ===");

    let fast_config = TransactionConfig::fast();
    println!("Fast config - simulate: {}, skip_preflight: {}",
        fast_config.simulate_before_send,
        fast_config.skip_preflight);

    let reliable_config = TransactionConfig::reliable();
    println!("Reliable config - retries: {}, commitment: {:?}",
        reliable_config.max_retries,
        reliable_config.commitment);

    let custom_config = TransactionConfig::default()
        .with_compute_units(200_000)
        .with_priority_fee(5000);
    println!("Custom config - CU: {:?}, priority: {:?}",
        custom_config.compute_units,
        custom_config.priority_fee_micro_lamports);

    // Example 3: Batch execution (would require funded account)
    println!("\n=== Batch Executor Example (dry run) ===");

    let executor = BatchExecutor::new(rpc_url);

    // Create multiple transfer instructions
    let recipients: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
    let instructions: Vec<_> = recipients
        .iter()
        .map(|r| system_instruction::transfer(&payer.pubkey(), r, 1000))
        .collect();

    println!("Created {} transfer instructions", instructions.len());

    // Split into batches
    let batches = executor.split_into_batches(instructions.clone(), 2);
    println!("Split into {} batches", batches.len());

    // Example 4: Estimating transaction size
    println!("\n=== Transaction Size Estimation ===");

    use solana_pipkit::transaction::{estimate_transaction_size, will_fit_in_transaction};

    let size = estimate_transaction_size(&instructions[..1], 1);
    println!("Single transfer size: {} bytes", size);

    let fits = will_fit_in_transaction(&instructions, 1);
    println!("All 5 transfers fit in one tx: {}", fits);

    // Example 5: Using BatchResult
    println!("\n=== Batch Result Analysis ===");

    // Simulated result
    let result = BatchResult {
        successful: vec![solana_sdk::signature::Signature::default()],
        failed: vec![(1, "Simulated error".to_string())],
        instructions_processed: 3,
    };

    println!("All succeeded: {}", result.all_succeeded());
    println!("Success rate: {:.1}%", result.success_rate());
    println!("Instructions processed: {}", result.instructions_processed);

    println!("\n=== Example Complete ===");
    println!("Note: Actual transactions require a funded account");

    Ok(())
}
