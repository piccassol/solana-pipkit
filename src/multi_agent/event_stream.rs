//! Event subscription stream for multi-agent coordination.
//!
//! This module provides WebSocket/Geyser-based event subscriptions
//! that multiple agents can share for real-time updates.

use crate::{Result, ToolkitError};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Event types to subscribe to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Account change events.
    Account,
    /// Log events.
    Logs,
    /// Slot updates.
    Slots,
    /// Signature notifications.
    Signature,
    /// Program events.
    Program,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Account => write!(f, "account"),
            EventType::Logs => write!(f, "logs"),
            EventType::Slots => write!(f, "slots"),
            EventType::Signature => write!(f, "signature"),
            EventType::Program => write!(f, "program"),
        }
    }
}

/// Event filter for subscriptions.
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Event type to filter.
    pub event_type: EventType,
    /// Program/pubkey to filter (optional).
    pub target: Option<Pubkey>,
    /// Additional filters.
    pub filters: Vec<String>,
}

impl EventFilter {
    /// Create a new event filter.
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            target: None,
            filters: Vec::new(),
        }
    }

    /// Set target pubkey.
    pub fn with_target(mut self, pubkey: Pubkey) -> Self {
        self.target = Some(pubkey);
        self
    }

    /// Add a filter string.
    pub fn add_filter(mut self, filter: String) -> Self {
        self.filters.push(filter);
        self
    }
}

/// Solana event received from WebSocket.
#[derive(Debug, Clone)]
pub enum SolanaEvent {
    /// Account data changed.
    AccountChange {
        pubkey: Pubkey,
        data: Vec<u8>,
        slot: u64,
    },
    /// Program log emitted.
    Log {
        signature: Signature,
        logs: Vec<String>,
    },
    /// New slot processed.
    Slot(u64),
    /// Transaction confirmed.
    Signature {
        signature: Signature,
        status: bool,
    },
    /// Error occurred.
    Error(String),
    /// Connection state changed.
    Connected,
    Disconnected,
}

/// Event stream for real-time updates.
pub struct EventStream {
    url: String,
    next_id: AtomicU64,
    subscribers: Arc<RwLock<Vec<broadcast::Sender<SolanaEvent>>>>,
    active_subscriptions: Arc<RwLock<HashSet<u64>>>,
}

impl EventStream {
    /// Create a new event stream.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            next_id: AtomicU64::new(1),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            active_subscriptions: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<SolanaEvent> {
        let (tx, rx) = broadcast::channel(1000);
        let subscribers = self.subscribers.clone();
        tokio::spawn(async move {
            let mut subscribers_write = subscribers.write().await;
            subscribers_write.push(tx);
        });
        rx
    }

    /// Connect and start streaming events.
    pub async fn connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| ToolkitError::NetworkError(format!("WebSocket connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(event) = parse_ws_message(&text) {
                            let subs = subscribers.read().await;
                            for tx in subs.iter() {
                                let _ = tx.send(event.clone());
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let subs = subscribers.read().await;
                        for tx in subs.iter() {
                            let _ = tx.send(SolanaEvent::Disconnected);
                        }
                        break;
                    }
                    Err(e) => {
                        let subs = subscribers.read().await;
                        let error_event = SolanaEvent::Error(e.to_string());
                        for tx in subs.iter() {
                            let _ = tx.send(error_event.clone());
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });

        let subs = subscribers.read().await;
        for tx in subs.iter() {
            let _ = tx.send(SolanaEvent::Connected);
        }

        Ok(())
    }

    /// Subscribe to account changes.
    pub async fn subscribe_to_account(&self, pubkey: &Pubkey) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = SubscriptionRequest {
            jsonrpc: "2.0".to_string(),
            id: id as u64,
            method: "accountSubscribe".to_string(),
            params: serde_json::json!([
                pubkey.to_string(),
                {"encoding": "base64", "commitment": "confirmed"}
            ]),
        };

        self.send_request(&request).await?;

        let mut subs = self.active_subscriptions.write().await;
        subs.insert(id);

        Ok(id)
    }

    /// Subscribe to program logs.
    pub async fn subscribe_to_logs(&self, program_id: &Pubkey) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = SubscriptionRequest {
            jsonrpc: "2.0".to_string(),
            id: id as u64,
            method: "logsSubscribe".to_string(),
            params: serde_json::json!([
                {"mentions": [program_id.to_string()]},
                {"commitment": "confirmed"}
            ]),
        };

        self.send_request(&request).await?;

        let mut subs = self.active_subscriptions.write().await;
        subs.insert(id);

        Ok(id)
    }

    /// Subscribe to slot updates.
    pub async fn subscribe_to_slots(&self) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = SubscriptionRequest {
            jsonrpc: "2.0".to_string(),
            id: id as u64,
            method: "slotSubscribe".to_string(),
            params: serde_json::json!([]),
        };

        self.send_request(&request).await?;

        let mut subs = self.active_subscriptions.write().await;
        subs.insert(id);

        Ok(id)
    }

    /// Subscribe to signature notifications.
    pub async fn subscribe_to_signature(&self, signature: &Signature) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = SubscriptionRequest {
            jsonrpc: "2.0".to_string(),
            id: id as u64,
            method: "signatureSubscribe".to_string(),
            params: serde_json::json!([
                signature.to_string(),
                {"commitment": "confirmed"}
            ]),
        };

        self.send_request(&request).await?;

        let mut subs = self.active_subscriptions.write().await;
        subs.insert(id);

        Ok(id)
    }

    /// Send a subscription request.
    async fn send_request(&self, request: &SubscriptionRequest) -> Result<()> {
        let json = serde_json::to_string(request)
            .map_err(|e| ToolkitError::SerializationError(e.to_string()))?;

        // In a real implementation, we would send this over the WebSocket
        // For now, this is a placeholder
        tracing::debug!("Sending WebSocket request: {}", json);

        Ok(())
    }

    /// Unsubscribe from an event.
    pub async fn unsubscribe(&self, subscription_id: u64) -> Result<()> {
        let mut subs = self.active_subscriptions.write().await;
        subs.remove(&subscription_id);

        let request = SubscriptionRequest {
            jsonrpc: "2.0".to_string(),
            id: subscription_id as u64,
            method: "accountUnsubscribe".to_string(),
            params: serde_json::json!([subscription_id]),
        };

        self.send_request(&request).await?;

        Ok(())
    }

    /// Close the event stream.
    pub async fn close(&self) -> Result<()> {
        let subs = subscribers.read().await;
        for tx in subs.iter() {
            let _ = tx.send(SolanaEvent::Disconnected);
        }
        Ok(())
    }
}

/// WebSocket subscription request.
#[derive(Debug, Clone, Serialize)]
struct SubscriptionRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// WebSocket notification.
#[derive(Debug, Clone, Deserialize)]
struct WsNotification {
    jsonrpc: String,
    method: String,
    params: WsParams,
}

#[derive(Debug, Clone, Deserialize)]
struct WsParams {
    result: serde_json::Value,
    subscription: u64,
}

/// Parse WebSocket message into SolanaEvent.
fn parse_ws_message(message: &str) -> Option<SolanaEvent> {
    if let Ok(notification) = serde_json::from_str::<WsNotification>(message) {
        match notification.method.as_str() {
            "accountNotification" => {
                let result = notification.params.result;
                let pubkey = result.get("value")
                    .and_then(|v| v.get("pubkey"))
                    .and_then(|p| p.as_str())
                    .and_then(|s| s.parse().ok())?;

                let data = result.get("value")
                    .and_then(|v| v.get("data"))
                    .and_then(|d| d.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.as_str())
                    .and_then(|s| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok())?;

                let slot = result.get("context")
                    .and_then(|c| c.get("slot"))
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0);

                Some(SolanaEvent::AccountChange {
                    pubkey,
                    data,
                    slot,
                })
            }
            "logsNotification" => {
                let result = notification.params.result;
                let logs = result.get("value")
                    .and_then(|v| v.get("logs"))
                    .and_then(|l| l.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })?;

                let sig_str = result.get("value")
                    .and_then(|v| v.get("signature"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let signature = sig_str.parse().unwrap_or_default();

                Some(SolanaEvent::Log {
                    signature,
                    logs,
                })
            }
            "slotNotification" => {
                let slot = notification.params.result
                    .get("slot")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0);
                Some(SolanaEvent::Slot(slot))
            }
            "signatureNotification" => {
                let sig_str = notification.params.result
                    .get("value")
                    .and_then(|v| v.get("signature"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let signature = sig_str.parse().unwrap_or_default();

                let status = notification.params.result
                    .get("value")
                    .and_then(|v| v.get("err"))
                    .is_none();

                Some(SolanaEvent::Signature {
                    signature,
                    status,
                })
            }
            _ => None,
        }
    } else {
        None
    }
}

/// Multi-agent event relay that filters and routes events.
#[derive(Debug)]
pub struct EventRelay {
    stream: EventStream,
    filters: Vec<EventFilter>,
}

impl EventRelay {
    /// Create a new event relay.
    pub fn new(url: &str) -> Self {
        Self {
            stream: EventStream::new(url),
            filters: Vec::new(),
        }
    }

    /// Add a filter.
    pub fn add_filter(&mut self, filter: EventFilter) {
        self.filters.push(filter);
    }

    /// Start the relay and return a receiver.
    pub async fn start(&mut self) -> Result<broadcast::Receiver<SolanaEvent>> {
        self.stream.connect().await?;
        Ok(self.stream.subscribe())
    }

    /// Subscribe to a specific event type.
    pub async fn subscribe(&self, event_type: EventType) -> Result<u64> {
        match event_type {
            EventType::Account => {
                if let Some(filter) = self.filters.iter().find(|f| f.event_type == EventType::Account) {
                    if let Some(pubkey) = filter.target {
                        self.stream.subscribe_to_account(&pubkey).await
                    } else {
                        Err(ToolkitError::InvalidInput("Account subscription requires target pubkey".to_string()))
                    }
                } else {
                    Err(ToolkitError::InvalidInput("No account filter configured".to_string()))
                }
            }
            EventType::Logs => {
                if let Some(filter) = self.filters.iter().find(|f| f.event_type == EventType::Logs) {
                    if let Some(pubkey) = filter.target {
                        self.stream.subscribe_to_logs(&pubkey).await
                    } else {
                        Err(ToolkitError::InvalidInput("Logs subscription requires target program".to_string()))
                    }
                } else {
                    Err(ToolkitError::InvalidInput("No logs filter configured".to_string()))
                }
            }
            EventType::Slots => {
                self.stream.subscribe_to_slots().await
            }
            EventType::Signature => {
                Err(ToolkitError::InvalidInput("Use subscribe_to_signature() directly".to_string()))
            }
            EventType::Program => {
                if let Some(filter) = self.filters.iter().find(|f| f.event_type == EventType::Logs) {
                    if let Some(pubkey) = filter.target {
                        self.stream.subscribe_to_logs(&pubkey).await
                    } else {
                        Err(ToolkitError::InvalidInput("Program subscription requires target program".to_string()))
                    }
                } else {
                    Err(ToolkitError::InvalidInput("No program filter configured".to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_creation() {
        let filter = EventFilter::new(EventType::Account)
            .with_target(Pubkey::new_unique())
            .add_filter("data_length>0".to_string());

        assert_eq!(filter.event_type, EventType::Account);
        assert!(filter.target.is_some());
    }

    #[test]
    fn test_event_stream_creation() {
        let stream = EventStream::new("wss://api.mainnet-beta.solana.com");
        assert_eq!(stream.url, "wss://api.mainnet-beta.solana.com");
    }

    #[test]
    fn test_event_relay() {
        let mut relay = EventRelay::new("wss://api.mainnet-beta.solana.com");
        relay.add_filter(EventFilter::new(EventType::Account).with_target(Pubkey::new_unique()));
        relay.add_filter(EventFilter::new(EventType::Slots));

        assert_eq!(relay.filters.len(), 2);
    }
}
