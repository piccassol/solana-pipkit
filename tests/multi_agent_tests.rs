//! Integration tests for multi-agent module.

use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;

#[test]
#[cfg(feature = "multi-agent")]
fn test_agent_inbox_derivation() {
    use solana_pipkit::multi_agent::AgentInbox;

    let agent = Pubkey::new_unique();
    let (inbox, bump) = AgentInbox::derive(&agent);

    assert_ne!(inbox, Pubkey::default());
    assert!(bump > 0 && bump < 256);

    let inbox2 = AgentInbox::new(&agent);
    assert_eq!(inbox2.address, inbox);
    assert_eq!(inbox2.owner, agent);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_message_instruction() {
    use solana_pipkit::multi_agent::post_message;

    let to = Pubkey::new_unique();
    let from = Pubkey::new_unique();
    let payload = b"Test message from agent".to_vec();

    let instruction = post_message(&to, &from, payload.clone());

    assert_eq!(instruction.program_id, solana_pipkit::multi_agent::MULTI_AGENT_PROGRAM_ID);
    assert_eq!(instruction.accounts.len(), 3);
    assert!(!instruction.data.is_empty());
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_coordinator_registration() {
    use solana_pipkit::multi_agent::MultiAgentCoordinator;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let mut coordinator = MultiAgentCoordinator::new(client);

    let agent1 = Pubkey::new_unique();
    let agent2 = Pubkey::new_unique();

    coordinator.register_agent(&agent1);
    coordinator.register_agent(&agent2);

    let agents = coordinator.agents();
    assert_eq!(agents.len(), 2);
    assert!(agents.contains(&agent1));
    assert!(agents.contains(&agent2));

    assert!(coordinator.get_inbox(&agent1).is_some());
    assert!(coordinator.get_inbox(&agent2).is_some());
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_task_bid_creation() {
    use solana_pipkit::multi_agent::TaskBid;

    let task_id = Pubkey::new_unique();
    let bidder = Pubkey::new_unique();
    let amount = 1_000_000u64;

    let instruction = TaskBid::new(task_id, bidder, amount);

    assert_eq!(instruction.program_id, solana_pipkit::multi_agent::TASK_MARKETPLACE_PROGRAM_ID);
    assert_eq!(instruction.accounts.len(), 3);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_task_status_conversion() {
    use solana_pipkit::multi_agent::TaskStatus;

    assert_eq!(TaskStatus::from_u8(0), Some(TaskStatus::Open));
    assert_eq!(TaskStatus::from_u8(1), Some(TaskStatus::Assigned));
    assert_eq!(TaskStatus::from_u8(2), Some(TaskStatus::InProgress));
    assert_eq!(TaskStatus::from_u8(3), Some(TaskStatus::Completed));
    assert_eq!(TaskStatus::from_u8(4), Some(TaskStatus::Failed));
    assert_eq!(TaskStatus::from_u8(5), Some(TaskStatus::Cancelled));
    assert_eq!(TaskStatus::from_u8(99), None);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_event_filter() {
    use solana_pipkit::multi_agent::{EventFilter, EventType};

    let filter = EventFilter::new(EventType::Account)
        .with_target(Pubkey::new_unique())
        .add_filter("data_length>0".to_string());

    assert_eq!(filter.event_type, EventType::Account);
    assert!(filter.target.is_some());
    assert_eq!(filter.filters.len(), 1);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_encryptor_basic() {
    use solana_pipkit::multi_agent::{MessageEncryptor, EncryptionKey};

    let encryptor = MessageEncryptor::new();
    let plaintext = b"Secret agent message";
    let nonce = encryptor.generate_nonce();

    let ciphertext = encryptor.encrypt(plaintext, &nonce).unwrap();
    let decrypted = encryptor.decrypt(&ciphertext, &nonce).unwrap();

    assert_eq!(plaintext, decrypted.as_slice());
    assert_ne!(ciphertext, plaintext.to_vec());
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_encryptor_with_key() {
    use solana_pipkit::multi_agent::{MessageEncryptor, EncryptionKey};

    let key: EncryptionKey = [1u8; 32];
    let encryptor = MessageEncryptor::with_key(key);

    let plaintext = b"Message with specific key";
    let nonce = encryptor.generate_nonce();

    let ciphertext = encryptor.encrypt(plaintext, &nonce).unwrap();
    let decrypted = encryptor.decrypt(&ciphertext, &nonce).unwrap();

    assert_eq!(plaintext, decrypted.as_slice());
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_shared_key_derivation() {
    use solana_pipkit::multi_agent::MessageEncryptor;

    let key1: [u8; 32] = [1u8; 32];
    let key2: [u8; 32] = [2u8; 32];

    let shared1 = MessageEncryptor::derive_shared_key(&key1, &key2);
    let shared2 = MessageEncryptor::derive_shared_key(&key2, &key1);

    assert_eq!(shared1, shared2);
    assert_ne!(shared1, key1);
    assert_ne!(shared2, key2);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_agent_key_pair() {
    use solana_pipkit::multi_agent::AgentKeyPair;

    let keypair = AgentKeyPair::new();

    assert_ne!(keypair.public_key(), keypair.private_key());
    assert!(!keypair.public_key().iter().all(|&b| b == 0));
    assert!(!keypair.private_key().iter().all(|&b| b == 0));
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_shared_secret_manager() {
    use solana_pipkit::multi_agent::SharedSecretManager;

    let mut manager = SharedSecretManager::new();
    let secret = [42u8; 32];

    manager.store_secret("agent1", "agent2", secret);

    assert_eq!(manager.get_secret("agent1", "agent2"), Some(secret));
    assert_eq!(manager.get_secret("agent2", "agent1"), Some(secret));
    assert!(manager.remove_secret("agent1", "agent2"));
    assert_eq!(manager.get_secret("agent1", "agent2"), None);
}

#[test]
#[cfg(feature = "multi-agent")]
fn test_message_auth() {
    use solana_pipkit::multi_agent::MessageAuth;

    let message = b"Authenticated agent message";
    let key = [1u8; 32];

    let auth = MessageAuth::generate(message, &key);
    assert!(auth.verify(message, &key));
    assert!(!auth.verify(b"Different message", &key));
}
