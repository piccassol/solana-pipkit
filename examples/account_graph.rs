//! Example: Account Graph Traversal
//!
//! Demonstrates how to build and traverse account relationship graphs
//! for analyzing wallet holdings and finding closeable accounts.

use solana_pipkit::prelude::*;
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<()> {
    // Use mainnet for real data
    let rpc_url = "https://api.mainnet-beta.solana.com";

    // Example wallet with token accounts (use a real wallet for testing)
    // This is a placeholder - replace with actual wallet for real testing
    let wallet_str = "11111111111111111111111111111111"; // System program as placeholder
    let wallet = wallet_str.parse::<Pubkey>().unwrap();

    println!("=== Account Graph Example ===");
    println!("Wallet: {}", wallet);

    // Example 1: Building a graph manually
    println!("\n=== Manual Graph Construction ===");

    let mut graph = AccountGraph::new();

    // Add some example nodes
    let token_mint = Pubkey::new_unique();
    let token_account = Pubkey::new_unique();

    graph.add_node(AccountNode {
        pubkey: token_mint,
        owner: spl_token::id(),
        lamports: 1_461_600,
        data_len: 82,
        is_program: false,
        account_type: Some(AccountNodeType::TokenMint {
            supply: 1_000_000,
            decimals: 9,
        }),
    });

    graph.add_node(AccountNode {
        pubkey: token_account,
        owner: spl_token::id(),
        lamports: 2_039_280,
        data_len: 165,
        is_program: false,
        account_type: Some(AccountNodeType::TokenAccount {
            mint: token_mint,
            owner: wallet,
            amount: 0, // Empty!
        }),
    });

    // Add edge: token account -> mint
    graph.add_edge(AccountEdge {
        from: token_account,
        to: token_mint,
        edge_type: EdgeType::TokenAccountOf,
    });

    println!("Graph nodes: {}", graph.node_count());
    println!("Graph edges: {}", graph.edge_count());
    println!("Total lamports: {}", graph.total_lamports());

    // Example 2: Finding closeable accounts
    println!("\n=== Finding Closeable Accounts ===");

    use solana_pipkit::account_graph::utils;

    let closeable = utils::find_closeable_accounts(&graph);
    println!("Closeable accounts: {}", closeable.len());

    let recoverable = utils::total_recoverable_rent(&graph);
    println!("Recoverable rent: {} lamports ({:.6} SOL)",
        recoverable,
        recoverable as f64 / 1_000_000_000.0);

    // Example 3: Grouping accounts by owner
    println!("\n=== Accounts by Owner ===");

    let by_owner = utils::group_by_owner(&graph);
    for (owner, accounts) in &by_owner {
        println!("Owner {}: {} accounts", owner, accounts.len());
    }

    // Example 4: Graph traversal
    println!("\n=== Graph Traversal ===");

    let reachable = graph.find_reachable(&token_account);
    println!("Reachable from token account: {} nodes", reachable.len());

    let reaching = graph.find_reaching(&token_mint);
    println!("Nodes reaching token mint: {} nodes", reaching.len());

    // Example 5: Analyzing token accounts
    println!("\n=== Token Account Analysis ===");

    let token_accounts_for_mint = graph.token_accounts_for_mint(&token_mint);
    println!("Token accounts for mint: {}", token_accounts_for_mint.len());

    let token_accounts_for_wallet = graph.token_accounts_for_wallet(&wallet);
    println!("Token accounts for wallet: {}", token_accounts_for_wallet.len());

    // Example 6: Top accounts by balance
    println!("\n=== Top Accounts by Balance ===");

    let top_accounts = utils::top_accounts_by_balance(&graph, 5);
    for (i, account) in top_accounts.iter().enumerate() {
        println!("{}. {} - {} lamports",
            i + 1,
            account.pubkey,
            account.lamports);
    }

    // Example 7: Building graph from RPC (commented out to avoid rate limits)
    println!("\n=== Building Graph from RPC ===");
    println!("(Skipped to avoid rate limits - uncomment for real testing)");

    /*
    // Uncomment for real testing with a wallet that has token accounts
    let builder = AccountGraphBuilder::new(rpc_url);

    // Build token account graph for wallet
    let token_graph = builder.build_token_account_graph(&wallet).await?;
    println!("Token graph nodes: {}", token_graph.node_count());

    // Find all closeable accounts
    let closeable = utils::find_closeable_accounts(&token_graph);
    for account in closeable {
        println!("Closeable: {} ({} lamports)", account.pubkey, account.lamports);
    }
    */

    println!("\n=== Example Complete ===");

    Ok(())
}
