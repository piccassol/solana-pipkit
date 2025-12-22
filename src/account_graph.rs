//! Account graph traversal utilities.
//!
//! This module provides utilities for building and traversing
//! account relationship graphs on Solana.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{Result, ToolkitError};

/// Represents a node in the account graph.
#[derive(Debug, Clone)]
pub struct AccountNode {
    /// The account's public key.
    pub pubkey: Pubkey,
    /// The account's owner (program).
    pub owner: Pubkey,
    /// Lamports balance.
    pub lamports: u64,
    /// Data length.
    pub data_len: usize,
    /// Whether this is a program account.
    pub is_program: bool,
    /// Optional: parsed account type.
    pub account_type: Option<AccountNodeType>,
}

/// Type classification for account nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountNodeType {
    /// System program owned account.
    SystemAccount,
    /// SPL Token account.
    TokenAccount {
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
    },
    /// SPL Token mint.
    TokenMint {
        supply: u64,
        decimals: u8,
    },
    /// Metaplex metadata account.
    Metadata { mint: Pubkey },
    /// Program account.
    Program,
    /// Associated token account.
    AssociatedTokenAccount {
        wallet: Pubkey,
        mint: Pubkey,
    },
    /// Unknown account type.
    Unknown,
}

/// Edge type in the account graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// Program owns this account.
    OwnedBy,
    /// Token account for this mint.
    TokenAccountOf,
    /// Authority relationship.
    Authority,
    /// Metadata for token.
    MetadataOf,
    /// Associated token account relationship.
    AssociatedWith,
    /// Generic relationship.
    Related,
}

/// An edge in the account graph.
#[derive(Debug, Clone)]
pub struct AccountEdge {
    /// Source account.
    pub from: Pubkey,
    /// Target account.
    pub to: Pubkey,
    /// Type of relationship.
    pub edge_type: EdgeType,
}

/// Account graph for tracking relationships.
#[derive(Debug, Default)]
pub struct AccountGraph {
    /// All nodes in the graph.
    nodes: HashMap<Pubkey, AccountNode>,
    /// Edges indexed by source.
    edges_from: HashMap<Pubkey, Vec<AccountEdge>>,
    /// Edges indexed by target.
    edges_to: HashMap<Pubkey, Vec<AccountEdge>>,
}

impl AccountGraph {
    /// Create a new empty account graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: AccountNode) {
        self.nodes.insert(node.pubkey, node);
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: AccountEdge) {
        self.edges_from
            .entry(edge.from)
            .or_default()
            .push(edge.clone());
        self.edges_to.entry(edge.to).or_default().push(edge);
    }

    /// Get a node by pubkey.
    pub fn get_node(&self, pubkey: &Pubkey) -> Option<&AccountNode> {
        self.nodes.get(pubkey)
    }

    /// Get all edges from a node.
    pub fn edges_from(&self, pubkey: &Pubkey) -> &[AccountEdge] {
        self.edges_from.get(pubkey).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all edges to a node.
    pub fn edges_to(&self, pubkey: &Pubkey) -> &[AccountEdge] {
        self.edges_to.get(pubkey).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &AccountNode> {
        self.nodes.values()
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges_from.values().map(|v| v.len()).sum()
    }

    /// Find all accounts reachable from a starting node.
    pub fn find_reachable(&self, start: &Pubkey) -> HashSet<Pubkey> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(*start);

        while let Some(current) = queue.pop_front() {
            if visited.insert(current) {
                for edge in self.edges_from(&current) {
                    if !visited.contains(&edge.to) {
                        queue.push_back(edge.to);
                    }
                }
            }
        }

        visited
    }

    /// Find all accounts that can reach a target node.
    pub fn find_reaching(&self, target: &Pubkey) -> HashSet<Pubkey> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(*target);

        while let Some(current) = queue.pop_front() {
            if visited.insert(current) {
                for edge in self.edges_to(&current) {
                    if !visited.contains(&edge.from) {
                        queue.push_back(edge.from);
                    }
                }
            }
        }

        visited
    }

    /// Get all nodes of a specific type.
    pub fn nodes_of_type(&self, node_type: &AccountNodeType) -> Vec<&AccountNode> {
        self.nodes
            .values()
            .filter(|n| n.account_type.as_ref() == Some(node_type))
            .collect()
    }

    /// Find all token accounts for a specific mint.
    pub fn token_accounts_for_mint(&self, mint: &Pubkey) -> Vec<&AccountNode> {
        self.nodes
            .values()
            .filter(|n| {
                matches!(&n.account_type, Some(AccountNodeType::TokenAccount { mint: m, .. }) if m == mint)
            })
            .collect()
    }

    /// Find all token accounts owned by a wallet.
    pub fn token_accounts_for_wallet(&self, wallet: &Pubkey) -> Vec<&AccountNode> {
        self.nodes
            .values()
            .filter(|n| {
                matches!(&n.account_type, Some(AccountNodeType::TokenAccount { owner: o, .. }) if o == wallet)
            })
            .collect()
    }

    /// Calculate total lamports in the graph.
    pub fn total_lamports(&self) -> u64 {
        self.nodes.values().map(|n| n.lamports).sum()
    }

    /// Get nodes sorted by lamports (descending).
    pub fn nodes_by_lamports(&self) -> Vec<&AccountNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.lamports.cmp(&a.lamports));
        nodes
    }
}

/// Account graph builder for constructing graphs from on-chain data.
pub struct AccountGraphBuilder {
    client: RpcClient,
}

impl AccountGraphBuilder {
    /// Create a new graph builder.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    /// Create from an existing RPC client.
    pub fn from_client(client: RpcClient) -> Self {
        Self { client }
    }

    /// Build a graph from a list of account pubkeys.
    pub async fn build_from_accounts(&self, pubkeys: &[Pubkey]) -> Result<AccountGraph> {
        let mut graph = AccountGraph::new();

        // Fetch all accounts in batch
        let accounts = self.client.get_multiple_accounts(pubkeys).await?;

        for (pubkey, maybe_account) in pubkeys.iter().zip(accounts.iter()) {
            if let Some(account) = maybe_account {
                let node = self.create_node(*pubkey, account);
                graph.add_node(node);
            }
        }

        // Build edges based on account relationships
        self.build_edges(&mut graph);

        Ok(graph)
    }

    /// Build a graph of all token accounts for a wallet.
    pub async fn build_token_account_graph(&self, wallet: &Pubkey) -> Result<AccountGraph> {
        let mut graph = AccountGraph::new();
        let token_program = spl_token::id();

        // Get all token accounts
        let accounts = self
            .client
            .get_token_accounts_by_owner(
                wallet,
                solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program),
            )
            .await?;

        let mut pubkeys = Vec::new();
        let mut mints = HashSet::new();

        for keyed_account in &accounts {
            let pubkey = keyed_account.pubkey.parse::<Pubkey>().map_err(|e| {
                ToolkitError::Custom(format!("Failed to parse pubkey: {}", e))
            })?;
            pubkeys.push(pubkey);

            // Parse token account to get mint
            if let Some(account) = keyed_account.account.decode::<Account>() {
                if account.data.len() >= 32 {
                    let mint = Pubkey::try_from(&account.data[0..32])
                        .map_err(|_| ToolkitError::InvalidAccountData("Invalid mint".to_string()))?;
                    mints.insert(mint);
                }
            }
        }

        // Fetch token accounts
        let token_accounts = self.client.get_multiple_accounts(&pubkeys).await?;

        for (pubkey, maybe_account) in pubkeys.iter().zip(token_accounts.iter()) {
            if let Some(account) = maybe_account {
                let node = self.create_node(*pubkey, account);
                graph.add_node(node);
            }
        }

        // Fetch and add mints
        let mint_pubkeys: Vec<_> = mints.into_iter().collect();
        let mint_accounts = self.client.get_multiple_accounts(&mint_pubkeys).await?;

        for (pubkey, maybe_account) in mint_pubkeys.iter().zip(mint_accounts.iter()) {
            if let Some(account) = maybe_account {
                let node = self.create_node(*pubkey, account);
                graph.add_node(node);
            }
        }

        // Build edges
        self.build_edges(&mut graph);

        Ok(graph)
    }

    /// Create a node from account data.
    fn create_node(&self, pubkey: Pubkey, account: &Account) -> AccountNode {
        let account_type = self.classify_account(account);

        AccountNode {
            pubkey,
            owner: account.owner,
            lamports: account.lamports,
            data_len: account.data.len(),
            is_program: account.executable,
            account_type: Some(account_type),
        }
    }

    /// Classify an account based on its data and owner.
    fn classify_account(&self, account: &Account) -> AccountNodeType {
        let owner = account.owner;

        // System program account
        if owner == solana_sdk::system_program::id() {
            return AccountNodeType::SystemAccount;
        }

        // Token program accounts
        if owner == spl_token::id() {
            // Token account (165 bytes) vs Mint (82 bytes)
            if account.data.len() == 165 {
                // Parse token account
                if let Ok(token_account) = self.parse_token_account(&account.data) {
                    return token_account;
                }
            } else if account.data.len() == 82 {
                // Parse mint
                if let Ok(mint) = self.parse_mint(&account.data) {
                    return mint;
                }
            }
        }

        // Metaplex metadata (variable length, starts with specific discriminator)
        if owner == mpl_token_metadata::ID && !account.data.is_empty() {
            if account.data[0] == 4 && account.data.len() >= 33 {
                // Metadata account
                if let Ok(mint) = Pubkey::try_from(&account.data[1..33]) {
                    return AccountNodeType::Metadata { mint };
                }
            }
        }

        // Program account
        if account.executable {
            return AccountNodeType::Program;
        }

        AccountNodeType::Unknown
    }

    /// Parse SPL token account data.
    fn parse_token_account(&self, data: &[u8]) -> Result<AccountNodeType> {
        if data.len() < 72 {
            return Err(ToolkitError::InvalidAccountData("Data too short".to_string()));
        }

        let mint = Pubkey::try_from(&data[0..32])
            .map_err(|_| ToolkitError::InvalidAccountData("Invalid mint".to_string()))?;
        let owner = Pubkey::try_from(&data[32..64])
            .map_err(|_| ToolkitError::InvalidAccountData("Invalid owner".to_string()))?;
        let amount = u64::from_le_bytes(
            data[64..72].try_into().map_err(|_| ToolkitError::InvalidAccountData("Invalid amount".to_string()))?
        );

        Ok(AccountNodeType::TokenAccount { mint, owner, amount })
    }

    /// Parse SPL mint data.
    fn parse_mint(&self, data: &[u8]) -> Result<AccountNodeType> {
        if data.len() < 45 {
            return Err(ToolkitError::InvalidAccountData("Data too short".to_string()));
        }

        // Supply is at offset 36-44
        let supply = u64::from_le_bytes(
            data[36..44].try_into().map_err(|_| ToolkitError::InvalidAccountData("Invalid supply".to_string()))?
        );
        // Decimals at offset 44
        let decimals = data[44];

        Ok(AccountNodeType::TokenMint { supply, decimals })
    }

    /// Build edges based on account relationships.
    fn build_edges(&self, graph: &mut AccountGraph) {
        let nodes: Vec<_> = graph.nodes.values().cloned().collect();

        for node in &nodes {
            // Add owner edge
            if graph.nodes.contains_key(&node.owner) {
                graph.add_edge(AccountEdge {
                    from: node.pubkey,
                    to: node.owner,
                    edge_type: EdgeType::OwnedBy,
                });
            }

            // Add token-specific edges
            if let Some(AccountNodeType::TokenAccount { mint, owner, .. }) = &node.account_type {
                // Edge to mint
                if graph.nodes.contains_key(mint) {
                    graph.add_edge(AccountEdge {
                        from: node.pubkey,
                        to: *mint,
                        edge_type: EdgeType::TokenAccountOf,
                    });
                }

                // Edge to wallet owner
                if graph.nodes.contains_key(owner) {
                    graph.add_edge(AccountEdge {
                        from: node.pubkey,
                        to: *owner,
                        edge_type: EdgeType::Authority,
                    });
                }
            }

            // Add metadata edges
            if let Some(AccountNodeType::Metadata { mint }) = &node.account_type {
                if graph.nodes.contains_key(mint) {
                    graph.add_edge(AccountEdge {
                        from: node.pubkey,
                        to: *mint,
                        edge_type: EdgeType::MetadataOf,
                    });
                }
            }
        }
    }
}

/// Utility functions for account graph operations.
pub mod utils {
    use super::*;

    /// Find all closeable accounts in a graph (empty token accounts).
    pub fn find_closeable_accounts(graph: &AccountGraph) -> Vec<&AccountNode> {
        graph
            .nodes()
            .filter(|n| {
                matches!(
                    &n.account_type,
                    Some(AccountNodeType::TokenAccount { amount: 0, .. })
                )
            })
            .collect()
    }

    /// Calculate total recoverable rent from closeable accounts.
    pub fn total_recoverable_rent(graph: &AccountGraph) -> u64 {
        find_closeable_accounts(graph)
            .iter()
            .map(|n| n.lamports)
            .sum()
    }

    /// Group accounts by their owner program.
    pub fn group_by_owner(graph: &AccountGraph) -> HashMap<Pubkey, Vec<&AccountNode>> {
        let mut groups: HashMap<Pubkey, Vec<&AccountNode>> = HashMap::new();
        for node in graph.nodes() {
            groups.entry(node.owner).or_default().push(node);
        }
        groups
    }

    /// Find accounts with the largest balances.
    pub fn top_accounts_by_balance(graph: &AccountGraph, limit: usize) -> Vec<&AccountNode> {
        let mut nodes = graph.nodes_by_lamports();
        nodes.truncate(limit);
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_graph_new() {
        let graph = AccountGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = AccountGraph::new();
        let pubkey = Pubkey::new_unique();

        graph.add_node(AccountNode {
            pubkey,
            owner: solana_sdk::system_program::id(),
            lamports: 1000,
            data_len: 0,
            is_program: false,
            account_type: Some(AccountNodeType::SystemAccount),
        });

        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node(&pubkey).is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = AccountGraph::new();
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();

        graph.add_edge(AccountEdge {
            from,
            to,
            edge_type: EdgeType::OwnedBy,
        });

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.edges_from(&from).len(), 1);
        assert_eq!(graph.edges_to(&to).len(), 1);
    }

    #[test]
    fn test_find_reachable() {
        let mut graph = AccountGraph::new();
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let c = Pubkey::new_unique();

        graph.add_edge(AccountEdge {
            from: a,
            to: b,
            edge_type: EdgeType::Related,
        });
        graph.add_edge(AccountEdge {
            from: b,
            to: c,
            edge_type: EdgeType::Related,
        });

        let reachable = graph.find_reachable(&a);
        assert!(reachable.contains(&a));
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&c));
    }

    #[test]
    fn test_total_lamports() {
        let mut graph = AccountGraph::new();

        for i in 0..3 {
            graph.add_node(AccountNode {
                pubkey: Pubkey::new_unique(),
                owner: solana_sdk::system_program::id(),
                lamports: 1000 * (i + 1) as u64,
                data_len: 0,
                is_program: false,
                account_type: None,
            });
        }

        assert_eq!(graph.total_lamports(), 6000);
    }
}
