//! Integration tests for agent_analytics module.

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[test]
#[cfg(feature = "agent-analytics")]
fn test_interaction_graph_creation() {
    use solana_pipkit::agent_analytics::{InteractionGraph, InteractionType};

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
#[cfg(feature = "agent-analytics")]
fn test_interaction_graph_degrees() {
    use solana_pipkit::agent_analytics::{InteractionGraph, InteractionType};

    let mut graph = InteractionGraph::new();
    let hub = Pubkey::new_unique();
    let leaf1 = Pubkey::new_unique();
    let leaf2 = Pubkey::new_unique();
    let leaf3 = Pubkey::new_unique();

    graph.add_interaction(hub, leaf1, 100000, InteractionType::Transfer, 123456);
    graph.add_interaction(hub, leaf2, 100000, InteractionType::Transfer, 123457);
    graph.add_interaction(hub, leaf3, 100000, InteractionType::Transfer, 123458);

    assert_eq!(graph.degree_centrality(&hub), 3);
    assert_eq!(graph.degree_centrality(&leaf1), 1);
    assert_eq!(graph.get_outgoing(&hub).len(), 3);
    assert_eq!(graph.get_incoming(&leaf1).len(), 1);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_agent_interactions() {
    use solana_pipkit::agent_analytics::{InteractionGraph, InteractionType};

    let mut graph = InteractionGraph::new();
    let agent1 = Pubkey::new_unique();
    let agent2 = Pubkey::new_unique();

    graph.add_interaction(agent1, agent2, 1000000, InteractionType::Transfer, 123456);
    graph.add_interaction(agent1, agent2, 500000, InteractionType::Swap, 123457);
    graph.add_interaction(agent2, agent1, 2000000, InteractionType::CPI, 123458);

    let agent1_interactions = graph.get_agent_interactions(&agent1);
    assert_eq!(agent1_interactions.len(), 3);

    let outgoing = graph.get_outgoing(&agent1);
    assert_eq!(outgoing.len(), 2);

    let incoming = graph.get_incoming(&agent1);
    assert_eq!(incoming.len(), 1);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_risk_score_calculation() {
    use solana_pipkit::agent_analytics::RiskScore;

    let mut risk = RiskScore {
        rug_pull_risk: 0.3,
        failure_rate_risk: 0.2,
        concentration_risk: 0.4,
        bad_actor_risk: 0.1,
        overall_score: 0.0,
        risk_level: solana_pipkit::agent_analytics::RiskLevel::Low,
        factors: vec![],
    };

    risk.overall_score = risk.calculate();

    assert!(risk.overall_score > 0.0 && risk.overall_score < 1.0);

    risk.risk_level = solana_pipkit::agent_analytics::RiskScore::level_from_score(0.75);
    assert_eq!(risk.risk_level, solana_pipkit::agent_analytics::RiskLevel::High);

    risk.risk_level = solana_pipkit::agent_analytics::RiskScore::level_from_score(0.15);
    assert_eq!(risk.risk_level, solana_pipkit::agent_analytics::RiskLevel::Low);

    risk.risk_level = solana_pipkit::agent_analytics::RiskScore::level_from_score(0.85);
    assert_eq!(risk.risk_level, solana_pipkit::agent_analytics::RiskLevel::Critical);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_agent_type_display() {
    use solana_pipkit::agent_analytics::{AgentType, BotStrategy};

    let bot = AgentType::Bot(BotStrategy::Sniper);
    assert_eq!(bot.as_str(), "Bot");

    let human = AgentType::Human;
    assert_eq!(human.as_str(), "Human");

    let mev = AgentType::MEVSearcher;
    assert_eq!(mev.as_str(), "MEVSearcher");
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_bot_strategy_variants() {
    use solana_pipkit::agent_analytics::{AgentType, BotStrategy};

    let sniper = AgentType::Bot(BotStrategy::Sniper);
    let arb = AgentType::Bot(BotStrategy::Arbitrage);
    let mm = AgentType::Bot(BotStrategy::MarketMaker);

    assert_ne!(sniper, arb);
    assert_ne!(arb, mm);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_agent_flags() {
    use solana_pipkit::agent_analytics::AgentFlag;

    let whale = AgentFlag::Whale;
    let suspicious = AgentFlag::Suspicious;
    let rug = AgentFlag::RugPuller;

    assert_eq!(whale, AgentFlag::Whale);
    assert_ne!(whale, suspicious);
    assert_eq!(suspicious, AgentFlag::Suspicious);
    assert_eq!(rug, AgentFlag::RugPuller);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_risk_level_comparison() {
    use solana_pipkit::agent_analytics::RiskLevel;

    assert!(RiskLevel::Low < RiskLevel::Medium);
    assert!(RiskLevel::Medium < RiskLevel::High);
    assert!(RiskLevel::High < RiskLevel::Critical);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_interaction_types() {
    use solana_pipkit::agent_analytics::InteractionType;

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
#[cfg(feature = "agent-analytics")]
fn test_interaction_types_hash_set() {
    use solana_pipkit::agent_analytics::InteractionType;
    use std::collections::HashSet;

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
#[cfg(feature = "agent-analytics")]
fn test_empty_interaction_graph() {
    use solana_pipkit::agent_analytics::InteractionGraph;

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
#[cfg(feature = "agent-analytics")]
fn test_graph_path_detection() {
    use solana_pipkit::agent_analytics::{InteractionGraph, InteractionType};

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

#[test]
#[cfg(feature = "agent-analytics")]
fn test_top_coordinated_agents_limit() {
    use solana_client::rpc_client::RpcClient;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let address = Pubkey::new_unique();

    let result = solana_pipkit::agent_analytics::top_coordinated_agents(&client, &address, 5);

    assert!(result.is_ok());
    let top = result.unwrap();
    assert!(top.len() <= 5);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_classify_agent_structure() {
    use solana_client::rpc_client::RpcClient;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let address = Pubkey::new_unique();

    let result = solana_pipkit::agent_analytics::classify_agent(&client, &address, 10);

    assert!(result.is_ok());
    let profile = result.unwrap();
    assert_eq!(profile.address, address);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_agent_success_rate_range() {
    use solana_client::rpc_client::RpcClient;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let address = Pubkey::new_unique();

    let result = solana_pipkit::agent_analytics::agent_success_rate(&client, &address, 100);

    assert!(result.is_ok());
    let rate = result.unwrap();
    assert!(rate >= 0.0 && rate <= 100.0);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_agent_risk_score_structure() {
    use solana_client::rpc_client::RpcClient;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let address = Pubkey::new_unique();

    let result = solana_pipkit::agent_analytics::agent_risk_score(&client, &address);

    assert!(result.is_ok());
    let risk = result.unwrap();
    assert!(risk.overall_score >= 0.0 && risk.overall_score <= 1.0);
    assert!(risk.rug_pull_risk >= 0.0 && risk.rug_pull_risk <= 1.0);
    assert!(risk.failure_rate_risk >= 0.0 && risk.failure_rate_risk <= 1.0);
    assert!(risk.concentration_risk >= 0.0 && risk.concentration_risk <= 1.0);
    assert!(risk.bad_actor_risk >= 0.0 && risk.bad_actor_risk <= 1.0);
}

#[test]
#[cfg(feature = "agent-analytics")]
fn test_build_interaction_graph_empty() {
    use solana_client::rpc_client::RpcClient;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let addresses: Vec<Pubkey> = vec![];

    let result = solana_pipkit::agent_analytics::build_interaction_graph(&client, &addresses, 10);

    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.nodes.len(), 0);
    assert_eq!(graph.edges.len(), 0);
}
