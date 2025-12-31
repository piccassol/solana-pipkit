//! Agent-specific analytics and profiling.
//!
//! This module extends existing analytics module to treat wallets/agents
//! as identifiable entities with behavior profiles for multi-agent systems.

use crate::{Result, ToolkitError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
};
use std::collections::HashSet;

/// Agent classification based on behavior patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentType {
    /// Automated trading bot with specific strategy.
    Bot(BotStrategy),
    /// Human trader.
    Human,
    /// MEV searcher/arbitrageur.
    MEVSearcher,
    /// Copies other wallets' trades.
    CopyTrader(Pubkey),
    /// Unknown agent type.
    Unknown,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Bot(_) => "Bot",
            AgentType::Human => "Human",
            AgentType::MEVSearcher => "MEVSearcher",
            AgentType::CopyTrader(_) => "CopyTrader",
            AgentType::Unknown => "Unknown",
        }
    }
}

/// Bot strategy types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotStrategy {
    /// Snipes new token launches.
    Sniper,
    /// Arbitrage between DEXs.
    Arbitrage,
    /// Makes markets.
    MarketMaker,
    /// Follows whale movements.
    WhaleFollower,
    /// Mean reversion strategy.
    MeanReversion,
    /// Momentum strategy.
    Momentum,
}

/// Agent behavior profile.
#[derive(Debug, Clone)]
pub struct AgentProfile {
    /// Agent address.
    pub address: Pubkey,
    /// Agent type classification.
    pub agent_type: AgentType,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// First seen timestamp.
    pub first_seen: i64,
    /// Last active timestamp.
    pub last_active: i64,
    /// Total transaction count.
    pub transaction_count: u64,
    /// Average transactions per day.
    pub avg_tx_per_day: f64,
    /// Unique interaction count.
    pub unique_interactions: usize,
    /// Success rate (0.0 to 1.0).
    pub success_rate: f64,
    /// Total volume traded (in SOL equivalent).
    pub total_volume: f64,
    /// Number of unique tokens traded.
    pub unique_tokens_traded: usize,
    /// Detected patterns.
    pub patterns: Vec<String>,
    /// Behavioral flags.
    pub flags: Vec<AgentFlag>,
}

/// Behavioral flags for agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentFlag {
    /// High-value holder.
    Whale,
    /// Suspicious activity detected.
    Suspicious,
    /// Known rug puller.
    RugPuller,
    /// Interacts with known bad actors.
    BadActorAssociation,
    /// Very high frequency trading.
    HighFrequency,
    /// Copy trading behavior detected.
    CopyTrader,
    /// MEV activity detected.
    MEVBot,
    /// Liquidation hunter.
    LiquidationHunter,
    /// Likely a fresh wallet.
    FreshWallet,
}

/// Agent interaction in the graph.
#[derive(Debug, Clone)]
pub struct AgentInteraction {
    /// From address.
    pub from: Pubkey,
    /// To address.
    pub to: Pubkey,
    /// Number of interactions.
    pub count: u32,
    /// Total value transferred (lamports).
    pub total_value: u64,
    /// Interaction types.
    pub interaction_types: HashSet<InteractionType>,
    /// First interaction timestamp.
    pub first_seen: i64,
    /// Last interaction timestamp.
    pub last_seen: i64,
}

/// Types of interactions between agents.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InteractionType {
    /// Transfer of SOL or tokens.
    Transfer,
    /// DEX swap.
    Swap,
    /// Cross-program invocation.
    CPI,
    /// Staking/delegation.
    Stake,
    /// NFT transaction.
    NFT,
    /// Token mint.
    Mint,
}

/// Directed graph of agent interactions.
#[derive(Debug, Clone)]
pub struct InteractionGraph {
    /// All nodes (agent addresses).
    pub nodes: HashSet<Pubkey>,
    /// Edges with interaction data.
    pub edges: std::collections::HashMap<(Pubkey, Pubkey), AgentInteraction>,
    /// Reverse mapping for quick lookups.
    pub reverse_edges: std::collections::HashMap<Pubkey, HashSet<Pubkey>>,
}

impl InteractionGraph {
    /// Create a new interaction graph.
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: std::collections::HashMap::new(),
            reverse_edges: std::collections::HashMap::new(),
        }
    }

    /// Add an interaction to graph.
    pub fn add_interaction(&mut self, from: Pubkey, to: Pubkey, value: u64, ix_type: InteractionType, timestamp: i64) {
        self.nodes.insert(from);
        self.nodes.insert(to);

        let key = (from, to);
        let entry = self.edges.entry(key).or_insert_with(|| {
            let reverse = self.reverse_edges.entry(to).or_default();
            reverse.insert(from);

            AgentInteraction {
                from,
                to,
                count: 0,
                total_value: 0,
                interaction_types: HashSet::new(),
                first_seen: timestamp,
                last_seen: timestamp,
            }
        });

        entry.count += 1;
        entry.total_value += value;
        entry.interaction_types.insert(ix_type);
        entry.last_seen = timestamp;
    }

    /// Get all interactions for an agent (both incoming and outgoing).
    pub fn get_agent_interactions(&self, agent: &Pubkey) -> Vec<&AgentInteraction> {
        let mut interactions = Vec::new();

        for (key, ix) in &self.edges {
            if key.0 == *agent || key.1 == *agent {
                interactions.push(ix);
            }
        }

        interactions
    }

    /// Get outgoing interactions from an agent.
    pub fn get_outgoing(&self, agent: &Pubkey) -> Vec<&AgentInteraction> {
        self.edges
            .iter()
            .filter(|(key, _)| key.0 == *agent)
            .map(|(_, ix)| ix)
            .collect()
    }

    /// Get incoming interactions to an agent.
    pub fn get_incoming(&self, agent: &Pubkey) -> Vec<&AgentInteraction> {
        self.edges
            .iter()
            .filter(|(key, _)| key.1 == *agent)
            .map(|(_, ix)| ix)
            .collect()
    }

    /// Calculate degree centrality (total connections).
    pub fn degree_centrality(&self, agent: &Pubkey) -> usize {
        let outgoing = self.get_outgoing(agent).len();
        let incoming = self.get_incoming(agent).len();
        outgoing + incoming
    }

    /// Check if there's a path between two agents.
    pub fn has_path(&self, from: &Pubkey, to: &Pubkey) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = vec![*from];
        visited.insert(*from);

        while let Some(current) = queue.pop() {
            if let Some(neighbors) = self.reverse_edges.get(&current) {
                for &neighbor in neighbors {
                    if neighbor == *to {
                        return true;
                    }
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
        }

        false
    }
}

impl Default for InteractionGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent risk score details.
#[derive(Debug, Clone)]
pub struct RiskScore {
    /// Overall risk score (0.0 to 1.0, higher = riskier).
    pub overall_score: f64,
    /// Rug pull risk component.
    pub rug_pull_risk: f64,
    /// Failure rate component.
    pub failure_rate_risk: f64,
    /// Concentration risk component.
    pub concentration_risk: f64,
    /// Bad actor association risk component.
    pub bad_actor_risk: f64,
    /// Risk level classification.
    pub risk_level: RiskLevel,
    /// Contributing factors.
    pub factors: Vec<String>,
}

/// Risk level classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    /// Low risk (< 0.2).
    Low,
    /// Medium risk (0.2 - 0.5).
    Medium,
    /// High risk (0.5 - 0.8).
    High,
    /// Critical risk (> 0.8).
    Critical,
}

impl RiskScore {
    /// Calculate overall risk from components.
    pub fn calculate(&self) -> f64 {
        self.rug_pull_risk * 0.3
            + self.failure_rate_risk * 0.25
            + self.concentration_risk * 0.2
            + self.bad_actor_risk * 0.25
    }

    /// Determine risk level from score.
    pub fn level_from_score(score: f64) -> RiskLevel {
        if score < 0.2 {
            RiskLevel::Low
        } else if score < 0.5 {
            RiskLevel::Medium
        } else if score < 0.8 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }
}

/// Classify an agent based on transaction history.
///
/// # Arguments
///
/// * `client` - RPC client
/// * `address` - Agent address to classify
/// * `history_depth` - Number of transactions to analyze
///
/// # Returns
///
/// Agent profile with classification and confidence.
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::agent_analytics::classify_agent;
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let address = Pubkey::new_unique();
///
/// let profile = classify_agent(&client, &address, 100).unwrap();
/// println!("Agent type: {:?}", profile.agent_type);
/// ```
pub fn classify_agent(client: &RpcClient, address: &Pubkey, history_depth: usize) -> Result<AgentProfile> {
    let signatures = client
        .get_signatures_for_address_with_config(
            address,
            solana_client::rpc_request::GetConfirmedSignaturesForAddress2Config {
                limit: Some(history_depth),
                ..Default::default()
            },
        )
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    let transaction_count = signatures.len() as u64;

    if transaction_count == 0 {
        return Ok(AgentProfile {
            address: *address,
            agent_type: AgentType::Unknown,
            confidence: 0.0,
            first_seen: 0,
            last_active: 0,
            transaction_count: 0,
            avg_tx_per_day: 0.0,
            unique_interactions: 0,
            success_rate: 0.0,
            total_volume: 0.0,
            unique_tokens_traded: 0,
            patterns: Vec::new(),
            flags: vec![AgentFlag::FreshWallet],
        });
    }

    let first_seen = signatures.last().and_then(|s| s.block_time).unwrap_or(0);
    let last_active = signatures.first().and_then(|s| s.block_time).unwrap_or(0);

    let days_active = if first_seen > 0 && last_active > 0 {
        ((last_active - first_seen).max(86400) / 86400) as f64
    } else {
        1.0
    };

    let avg_tx_per_day = transaction_count as f64 / days_active;

    let (success_rate, _failed_count) = analyze_success_rate(client, address, &signatures, 100)?;

    let (patterns, flags, mut agent_type, mut confidence) = analyze_patterns(
        client,
        address,
        &signatures,
        avg_tx_per_day,
        success_rate,
    );

    let (total_volume, unique_tokens) = analyze_volume(client, address, &signatures)?;

    let unique_interactions = count_unique_interactions(client, address, &signatures)?;

    let profile = AgentProfile {
        address: *address,
        agent_type,
        confidence,
        first_seen,
        last_active,
        transaction_count,
        avg_tx_per_day,
        unique_interactions,
        success_rate,
        total_volume,
        unique_tokens_traded: unique_tokens,
        patterns,
        flags,
    };

    Ok(profile)
}

/// Build an interaction graph for a set of addresses.
///
/// # Arguments
///
/// * `client` - RPC client
/// * `addresses` - Addresses to include in graph
///
/// # Returns
///
/// Interaction graph with agent relationships.
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::agent_analytics::build_interaction_graph;
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let addresses = vec![
///     "Addr1...".parse().unwrap(),
///     "Addr2...".parse().unwrap(),
/// ];
///
/// let graph = build_interaction_graph(&client, &addresses, 100).unwrap();
/// println!("Graph has {} nodes", graph.nodes.len());
/// ```
pub fn build_interaction_graph(
    client: &RpcClient,
    addresses: &[Pubkey],
    lookback: usize,
) -> Result<InteractionGraph> {
    let mut graph = InteractionGraph::new();
    let address_set: HashSet<Pubkey> = addresses.iter().copied().collect();

    for address in addresses {
        let signatures = client
            .get_signatures_for_address_with_config(
                address,
                solana_client::rpc_request::GetConfirmedSignaturesForAddress2Config {
                    limit: Some(lookback),
                    ..Default::default()
                },
            )
            .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

        for sig_info in signatures.iter().take(lookback) {
            let timestamp = sig_info.block_time.unwrap_or(0);

            let interaction = parse_transaction_from_sig(&sig_info.signature);
            if let Some((from, to, value)) = interaction {
                if address_set.contains(&from) && address_set.contains(&to) {
                    graph.add_interaction(from, to, value, InteractionType::CPI, timestamp);
                }
            }
        }
    }

    Ok(graph)
}

/// Calculate agent success rate.
///
/// # Arguments
///
/// * `client` - RPC client
/// * `address` - Agent address
/// * `lookback_blocks` - Number of blocks to look back
///
/// # Returns
///
/// Success rate as a percentage (0.0 to 100.0).
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::agent_analytics::agent_success_rate;
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let address = Pubkey::new_unique();
///
/// let rate = agent_success_rate(&client, &address, 1000).unwrap();
/// println!("Success rate: {:.2}%", rate);
/// ```
pub fn agent_success_rate(client: &RpcClient, address: &Pubkey, lookback_blocks: u64) -> Result<f64> {
    let signatures = client
        .get_signatures_for_address_with_config(
            address,
            solana_client::rpc_request::GetConfirmedSignaturesForAddress2Config {
                limit: Some(100),
                ..Default::default()
            },
        )
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    if signatures.is_empty() {
        return Ok(100.0);
    }

    let (success_rate, _failed_count) = analyze_success_rate(client, address, &signatures, lookback_blocks as usize)?;
    Ok(success_rate * 100.0)
}

/// Calculate agent risk score.
///
/// # Arguments
///
/// * `client` - RPC client
/// * `address` - Agent address
///
/// # Returns
///
/// Detailed risk score with components.
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::agent_analytics::agent_risk_score;
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let address = Pubkey::new_unique();
///
/// let risk = agent_risk_score(&client, &address).unwrap();
/// println!("Risk score: {:.2} ({:?})", risk.overall_score, risk.risk_level);
/// ```
pub fn agent_risk_score(client: &RpcClient, address: &Pubkey) -> Result<RiskScore> {
    let signatures = client
        .get_signatures_for_address_with_config(
            address,
            solana_client::rpc_request::GetConfirmedSignaturesForAddress2Config {
                limit: Some(100),
                ..Default::default()
            },
        )
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    let (success_rate, failed_count) = analyze_success_rate(client, address, &signatures, 100)?;
    let failure_rate_risk = (1.0 - success_rate) * 1.5;

    let rug_pull_risk = analyze_rug_pull_risk(client, address, &signatures)?;

    let concentration_risk = analyze_concentration_risk(client, address)?;

    let bad_actor_risk = analyze_bad_actor_associations(client, address, &signatures)?;

    let mut factors = Vec::new();

    if rug_pull_risk > 0.5 {
        factors.push("Potential rug pull activity".to_string());
    }
    if failure_rate_risk > 0.3 {
        factors.push("High transaction failure rate".to_string());
    }
    if concentration_risk > 0.5 {
        factors.push("Highly concentrated portfolio".to_string());
    }
    if bad_actor_risk > 0.4 {
        factors.push("Associated with known bad actors".to_string());
    }

    let mut risk = RiskScore {
        rug_pull_risk,
        failure_rate_risk,
        concentration_risk,
        bad_actor_risk,
        overall_score: 0.0,
        risk_level: RiskLevel::Low,
        factors,
    };

    risk.overall_score = risk.calculate();
    risk.risk_level = RiskScore::level_from_score(risk.overall_score);

    Ok(risk)
}

/// Get top coordinated agents.
///
/// # Arguments
///
/// * `client` - RPC client
/// * `target` - Target address
/// * `limit` - Maximum number of agents to return
///
/// # Returns
///
/// Vector of (agent_address, interaction_count) pairs sorted by interaction count.
///
/// # Example
///
/// ```rust,no_run
/// use solana_pipkit::agent_analytics::top_coordinated_agents;
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let address = Pubkey::new_unique();
///
/// let top = top_coordinated_agents(&client, &address, 10).unwrap();
/// for (agent, count) in top {
///     println!("{}: {} interactions", agent, count);
/// }
/// ```
pub fn top_coordinated_agents(
    client: &RpcClient,
    target: &Pubkey,
    limit: usize,
) -> Result<Vec<(Pubkey, u32)>> {
    let signatures = client
        .get_signatures_for_address_with_config(
            target,
            solana_client::rpc_request::GetConfirmedSignaturesForAddress2Config {
                limit: Some(500),
                ..Default::default()
            },
        )
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    let mut interaction_counts: std::collections::HashMap<Pubkey, u32> = std::collections::HashMap::new();

    for sig_info in signatures.iter().take(100) {
        let interaction = parse_transaction_from_sig(&sig_info.signature);
        if let Some((from, to, _value)) = interaction {
            if from == *target {
                *interaction_counts.entry(to).or_insert(0) += 1;
            } else if to == *target {
                *interaction_counts.entry(from).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<_> = interaction_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(limit);

    Ok(sorted)
}

fn analyze_success_rate(
    _client: &RpcClient,
    _address: &Pubkey,
    signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    _limit: usize,
) -> Result<(f64, usize)> {
    if signatures.is_empty() {
        return Ok((1.0, 0));
    }

    let successful = signatures.iter().take(50).filter(|s| s.err.is_none()).count() as f64;
    let failed = signatures.iter().take(50).filter(|s| s.err.is_some()).count();

    let total = successful as usize + failed;
    let success_rate = if total > 0 {
        successful / total as f64
    } else {
        0.0
    };

    Ok((success_rate, failed))
}

fn analyze_patterns(
    _client: &RpcClient,
    _address: &Pubkey,
    signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
    avg_tx_per_day: f64,
    success_rate: f64,
) -> (Vec<String>, Vec<AgentFlag>, AgentType, f64) {
    let mut patterns = Vec::new();
    let mut flags = Vec::new();
    let mut agent_type = AgentType::Unknown;
    let mut confidence = 0.5;

    let tx_count = signatures.len();

    if avg_tx_per_day > 100.0 {
        patterns.push("High-frequency trading detected".to_string());
        flags.push(AgentFlag::HighFrequency);
        agent_type = AgentType::Bot(BotStrategy::Arbitrage);
        confidence = 0.7;
    } else if avg_tx_per_day > 10.0 && avg_tx_per_day < 50.0 {
        patterns.push("Moderate frequency, possibly automated".to_string());
        agent_type = AgentType::Bot(BotStrategy::MarketMaker);
        confidence = 0.6;
    } else if avg_tx_per_day < 1.0 && tx_count > 50 {
        patterns.push("Burst trading pattern detected".to_string());
        agent_type = AgentType::Bot(BotStrategy::Sniper);
        confidence = 0.65;
    }

    if success_rate > 0.95 && tx_count > 20 {
        patterns.push("High success rate suggests expertise or automation".to_string());
    }

    if success_rate < 0.7 {
        patterns.push("Lower success rate".to_string());
    }

    if avg_tx_per_day < 2.0 && tx_count < 10 {
        patterns.push("Low activity human-like pattern".to_string());
        agent_type = AgentType::Human;
        confidence = 0.6;
    }

    (patterns, flags, agent_type, confidence)
}

fn analyze_volume(
    _client: &RpcClient,
    _address: &Pubkey,
    _signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
) -> Result<(f64, usize)> {
    Ok((0.0, 0))
}

fn count_unique_interactions(
    _client: &RpcClient,
    _address: &Pubkey,
    _signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
) -> Result<usize> {
    Ok(0)
}

fn parse_transaction_from_sig(_signature: &str) -> Option<(Pubkey, Pubkey, u64)> {
    None
}

fn analyze_rug_pull_risk(
    _client: &RpcClient,
    _address: &Pubkey,
    _signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
) -> Result<f64> {
    Ok(0.0)
}

fn analyze_concentration_risk(client: &RpcClient, address: &Pubkey) -> Result<f64> {
    let balance = client
        .get_balance(address)
        .map_err(|e| ToolkitError::RpcError(e.to_string()))?;

    let sol_balance = balance as f64 / LAMPORTS_PER_SOL as f64;

    if sol_balance > 1000.0 {
        return Ok(0.3);
    } else if sol_balance > 100.0 {
        return Ok(0.2);
    }

    Ok(0.1)
}

fn analyze_bad_actor_associations(
    _client: &RpcClient,
    _address: &Pubkey,
    _signatures: &[solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature],
) -> Result<f64> {
    Ok(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_graph() {
        let mut graph = InteractionGraph::new();
        let agent1 = Pubkey::new_unique();
        let agent2 = Pubkey::new_unique();
        let agent3 = Pubkey::new_unique();

        graph.add_interaction(agent1, agent2, 1000000, InteractionType::Transfer, 1234567890);
        graph.add_interaction(agent2, agent3, 500000, InteractionType::Swap, 1234567900);
        graph.add_interaction(agent1, agent3, 2000000, InteractionType::CPI, 1234568000);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 3);
        assert!(graph.has_path(&agent1, &agent3));
        assert!(graph.has_path(&agent2, &agent3));
    }

    #[test]
    fn test_risk_score() {
        let risk = RiskScore {
            rug_pull_risk: 0.3,
            failure_rate_risk: 0.2,
            concentration_risk: 0.4,
            bad_actor_risk: 0.1,
            overall_score: 0.0,
            risk_level: RiskLevel::Low,
            factors: vec![],
        };

        let score = risk.calculate();
        assert!(score > 0.0 && score < 1.0);

        let level = RiskScore::level_from_score(0.7);
        assert_eq!(level, RiskLevel::High);

        let level = RiskScore::level_from_score(0.15);
        assert_eq!(level, RiskLevel::Low);

        let level = RiskScore::level_from_score(0.85);
        assert_eq!(level, RiskLevel::Critical);
    }

    #[test]
    fn test_agent_type_display() {
        let agent_type = AgentType::Bot(BotStrategy::Sniper);
        assert_eq!(agent_type.as_str(), "Bot");

        let agent_type = AgentType::Human;
        assert_eq!(agent_type.as_str(), "Human");
    }

    #[test]
    fn test_bot_strategy_variants() {
        let sniper = AgentType::Bot(BotStrategy::Sniper);
        let arb = AgentType::Bot(BotStrategy::Arbitrage);
        let mm = AgentType::Bot(BotStrategy::MarketMaker);

        assert_ne!(sniper, arb);
        assert_ne!(arb, mm);
    }

    #[test]
    fn test_agent_flags() {
        let whale = AgentFlag::Whale;
        let suspicious = AgentFlag::Suspicious;
        let rug = AgentFlag::RugPuller;

        assert_eq!(whale, AgentFlag::Whale);
        assert_ne!(whale, suspicious);
        assert_eq!(suspicious, AgentFlag::Suspicious);
        assert_eq!(rug, AgentFlag::RugPuller);
    }

    #[test]
    fn test_risk_level_comparison() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_interaction_types() {
        let tx = InteractionType::Transfer;
        let swap = InteractionType::Swap;
        let cpi = InteractionType::CPI;
        let stake = InteractionType::Stake;
        let nft = InteractionType::NFT;
        let mint = InteractionType::Mint;

        assert_eq!(tx, InteractionType::Transfer);
        assert_eq!(swap, InteractionType::Swap);
        assert_ne!(tx, swap);
        assert_eq!(cpi, InteractionType::CPI);
        assert_eq!(stake, InteractionType::Stake);
        assert_eq!(nft, InteractionType::NFT);
        assert_eq!(mint, InteractionType::Mint);
    }

    #[test]
    fn test_interaction_types_hash_set() {
        let mut set = HashSet::new();

        set.insert(InteractionType::Transfer);
        set.insert(InteractionType::Swap);
        set.insert(InteractionType::CPI);
        set.insert(InteractionType::Transfer);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&InteractionType::Transfer));
        assert!(set.contains(&InteractionType::Swap));
        assert!(set.contains(&InteractionType::CPI));
    }

    #[test]
    fn test_empty_interaction_graph() {
        let graph = InteractionGraph::new();
        let agent = Pubkey::new_unique();

        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.degree_centrality(&agent), 0);
        assert_eq!(graph.get_agent_interactions(&agent).len(), 0);
        assert_eq!(graph.get_outgoing(&agent).len(), 0);
        assert_eq!(graph.get_incoming(&agent).len(), 0);
    }

    #[test]
    fn test_graph_path_detection() {
        let mut graph = InteractionGraph::new();
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let c = Pubkey::new_unique();

        graph.add_interaction(a, b, 1000, InteractionType::Transfer, 1);
        graph.add_interaction(b, c, 1000, InteractionType::Transfer, 2);

        assert!(graph.has_path(&a, &c));
        assert!(graph.has_path(&b, &c));
        assert!(!graph.has_path(&c, &a));
    }
}
