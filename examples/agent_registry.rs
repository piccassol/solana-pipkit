//! AgenC Agent Registry Example
//!
//! This example demonstrates a complete workflow for an AgenC edge agent:
//! 1. Register itself with capabilities and metadata
//! 2. Query and discover other agents
//! 3. Monitor registry events in real-time
//! 4. Integrate with the multi-agent module for task coordination

use solana_pipkit::agent_registry::{
    AgentRegistryClient, AgentFilter, RegistrationParams, UpdateParams,
};
use solana_sdk::signature::{Keypair, Signer};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AgenC Agent Registry Example ===\n");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    let client = AgentRegistryClient::new(rpc_url);

    let agent = Keypair::new();

    println!("📋 Agent Details:");
    println!("  Pubkey: {}", agent.pubkey());
    println!();

    register_agent(&client, &agent).await?;

    query_agents(&client).await?;

    discover_peers(&client).await?;

    update_agent(&client, &agent).await?;

    monitor_events(client).await?;

    Ok(())
}

async fn register_agent(
    client: &AgentRegistryClient,
    agent: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Step 1: Registering Agent");

    let capabilities = vec![
        "inference".to_string(),
        "model_finetuning".to_string(),
        "edge_computing".to_string(),
    ];

    let metadata = serde_json::to_vec(&serde_json::json!({
        "device_type": "jetson-nano",
        "gpu": "aarch64",
        "max_batch_size": 32,
        "supported_models": vec!["gpt-2", "bert"],
        "latency_ms": 150,
        "ram_gb": 8
    })).unwrap();

    let params = RegistrationParams::new(agent.pubkey(), capabilities, "1.0.0")
        .with_metadata(metadata);

    println!("  Capabilities: {:?}", params.capabilities);
    println!("  Version: {}", params.version);
    println!("  Metadata: {} bytes", params.metadata.as_ref().map(|m| m.len()).unwrap_or(0));

    match client.register_agent(&params, agent).await {
        Ok(signature) => println!("  ✅ Registered! Signature: {}\n", signature),
        Err(e) => {
            if e.to_string().contains("already registered") {
                println!("  ℹ️  Agent already registered, skipping...\n");
            } else {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

async fn query_agents(
    client: &AgentRegistryClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Step 2: Querying Agents");

    match client.get_agent_count().await {
        Ok(count) => println!("  Total registered agents: {}", count),
        Err(_) => println!("  Could not fetch total count"),
    }

    let filter = AgentFilter::new()
        .with_capability("inference")
        .with_min_reputation(5000)
        .active_only()
        .with_limit(10);

    match client.list_agents(&filter).await {
        Ok(result) => {
            println!("  Found {} inference agents (high reputation, active):", result.items.len());

            for (i, agent_info) in result.items.iter().enumerate() {
                println!("    {}. {} (reputation: {})", i + 1, agent_info.pubkey, agent_info.reputation);
                println!("       Capabilities: {:?}", agent_info.capabilities);
                println!("       Version: {}", agent_info.version);
            }
        }
        Err(e) => println!("  Error querying agents: {}", e),
    }

    println!();
    Ok(())
}

async fn discover_peers(
    client: &AgentRegistryClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Step 3: Discovering Peers for Collaborative Tasks");

    let required_capabilities = vec![
        "inference".to_string(),
        "model_finetuning".to_string(),
        "edge_computing".to_string(),
    ];

    let peers = client
        .discover_peers(required_capabilities, 5000, 5)
        .await?;

    println!("  Peers by capability:");

    for (capability, agents) in &peers {
        println!("    {}: {} available agents", capability, agents.len());
        for agent in agents {
            println!("      - {} (rep: {})", agent.pubkey, agent.reputation);
        }
    }

    println!();

    Ok(())
}

async fn update_agent(
    client: &AgentRegistryClient,
    agent: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("✏️  Step 4: Updating Agent Information");

    let new_metadata = serde_json::to_vec(&serde_json::json!({
        "device_type": "jetson-nano",
        "gpu": "aarch64",
        "max_batch_size": 64,
        "supported_models": vec!["gpt-2", "bert", "llama-7b"],
        "latency_ms": 120,
        "ram_gb": 8,
        "uptime_hours": 720
    })).unwrap();

    let params = UpdateParams::new()
        .with_capabilities(vec![
            "inference".to_string(),
            "model_finetuning".to_string(),
            "edge_computing".to_string(),
            "llm_serving".to_string(),
        ])
        .with_version("1.1.0")
        .with_metadata(new_metadata);

    match client.update_agent(&agent.pubkey(), &params, agent).await {
        Ok(signature) => println!("  ✅ Updated! Signature: {}", signature),
        Err(e) => {
            if e.to_string().contains("not found") {
                println!("  ℹ️  Agent not found, skipping update...");
            } else {
                println!("  ⚠️  Update failed: {}", e);
            }
        }
    }

    println!();
    Ok(())
}

async fn monitor_events(
    client: AgentRegistryClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("👂 Step 5: Monitoring Registry Events");
    println!("  Listening for agent registration/updates for 30 seconds...\n");

    let handle = client
        .subscribe_to_registry_events(move |event| async move {
            use solana_pipkit::agent_registry::RegistryEvent;

            match event {
                RegistryEvent::AgentRegistered { agent, owner } => {
                    println!("🆕 New Agent Registered:");
                    println!("    Agent: {}", agent);
                    println!("    Owner: {}", owner);
                }
                RegistryEvent::AgentUpdated { agent } => {
                    println!("🔄 Agent Updated: {}", agent);
                }
                RegistryEvent::AgentDeregistered { agent } => {
                    println!("❌ Agent Deregistered: {}", agent);
                }
                RegistryEvent::CapabilityChanged { agent, capability } => {
                    println!("✨ Capability Changed:");
                    println!("    Agent: {}", agent);
                    println!("    New capability: {}", capability);
                }
            }
        })
        .await?;

    tokio::time::sleep(Duration::from_secs(30)).await;
    handle.abort();

    println!("\n=== Example Complete ===");

    Ok(())
}

#[allow(dead_code)]
async fn demonstrate_multi_agent_integration(
    client: &AgentRegistryClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Multi-Agent Integration Example (Pseudo-code)");

    let peers = client
        .discover_peers(vec!["inference".to_string()], 5000, 3)
        .await?;

    if let Some(inference_agents) = peers.get("inference") {
        println!("  Found {} inference agents for potential collaboration", inference_agents.len());

        for peer in inference_agents {
            println!("  - {} (reputation: {})", peer.pubkey, peer.reputation);
            println!("    Capabilities: {:?}", peer.capabilities);

            // Pseudo-code for multi-agent integration:
            // 1. Derive peer's inbox PDA
            // 2. Create task payload
            // 3. Send encrypted message to peer's inbox
            // 4. Monitor for response
        }
    }

    Ok(())
}
