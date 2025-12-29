//! Real-time websocket listener for fastest reaction times.
//!
//! This module provides websocket subscriptions to Solana events for
//! real-time trading signals and instant transaction confirmation.
//!
//! # Features
//! - Account change subscriptions
//! - Program log subscriptions (for pool detection)
//! - Slot updates
//! - Transaction confirmation watching
//!
//! # Example
//! ```rust,no_run
//! use solana_pipkit::speed::SolanaWebsocket;
//! use solana_sdk::pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let ws = SolanaWebsocket::new("wss://api.mainnet-beta.solana.com");
//!
//!     // Subscribe to slot updates
//!     let subscription = ws.subscribe_slots(|slot| {
//!         println!("New slot: {}", slot);
//!     }).await?;
//!
//!     // Keep running...
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//!
//!     ws.unsubscribe(subscription).await?;
//!     Ok(())
//! }
//! ```

use crate::{Result, ToolkitError};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Subscription ID type.
pub type SubscriptionId = u64;

/// Websocket events.
#[derive(Debug, Clone)]
pub enum WebsocketEvent {
    /// Slot update.
    SlotUpdate(u64),
    /// Account data changed.
    AccountChange {
        /// Account pubkey.
        pubkey: Pubkey,
        /// New account data.
        data: Vec<u8>,
        /// Slot of the update.
        slot: u64,
    },
    /// Program logs.
    LogsUpdate {
        /// Transaction signature.
        signature: Signature,
        /// Log messages.
        logs: Vec<String>,
    },
    /// Transaction confirmed.
    TransactionConfirmed(Signature),
    /// Connection established.
    Connected,
    /// Connection lost.
    Disconnected,
    /// Error occurred.
    Error(String),
}

/// Websocket subscription request.
#[derive(Debug, Clone, Serialize)]
struct SubscribeRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// Websocket notification.
#[derive(Debug, Clone, Deserialize)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: NotificationParams,
}

/// Notification parameters.
#[derive(Debug, Clone, Deserialize)]
struct NotificationParams {
    result: serde_json::Value,
    subscription: u64,
}

/// Subscription response.
#[derive(Debug, Clone, Deserialize)]
struct SubscriptionResponse {
    jsonrpc: String,
    result: u64,
    id: u64,
}

/// Websocket error types.
#[derive(Debug, Clone)]
pub enum WsError {
    /// Connection failed.
    ConnectionFailed(String),
    /// Subscription failed.
    SubscriptionFailed(String),
    /// Send failed.
    SendFailed(String),
    /// Timeout.
    Timeout(String),
    /// Parse error.
    ParseError(String),
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            WsError::SubscriptionFailed(msg) => write!(f, "Subscription failed: {}", msg),
            WsError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            WsError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            WsError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for WsError {}

impl From<WsError> for ToolkitError {
    fn from(e: WsError) -> Self {
        ToolkitError::NetworkError(e.to_string())
    }
}

/// Callback type for event handlers.
pub type EventCallback = Box<dyn Fn(WebsocketEvent) + Send + Sync>;

/// Internal subscription state.
struct Subscription {
    id: SubscriptionId,
    callback: EventCallback,
}

/// Solana websocket client for real-time updates.
pub struct SolanaWebsocket {
    /// Websocket URL.
    url: String,
    /// Next request ID.
    next_id: AtomicU64,
    /// Active subscriptions.
    subscriptions: RwLock<HashMap<u64, Subscription>>,
    /// Command channel.
    command_tx: Option<mpsc::Sender<WsCommand>>,
    /// Connection state.
    connected: Arc<std::sync::atomic::AtomicBool>,
}

/// Internal commands.
enum WsCommand {
    Subscribe {
        method: String,
        params: serde_json::Value,
        response_tx: oneshot::Sender<Result<u64>>,
    },
    Unsubscribe {
        subscription_id: u64,
        response_tx: oneshot::Sender<Result<()>>,
    },
    SendMessage {
        message: String,
    },
}

impl SolanaWebsocket {
    /// Create a new websocket client.
    pub fn new(wss_url: &str) -> Self {
        Self {
            url: wss_url.to_string(),
            next_id: AtomicU64::new(1),
            subscriptions: RwLock::new(HashMap::new()),
            command_tx: None,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Connect to the websocket.
    pub async fn connect(&mut self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();
        let (command_tx, mut command_rx) = mpsc::channel::<WsCommand>(100);
        let (_event_tx, _event_rx) = mpsc::channel::<WebsocketEvent>(1000);

        self.command_tx = Some(command_tx);
        self.connected.store(true, Ordering::Relaxed);

        let connected = self.connected.clone();
        let subscriptions = Arc::new(RwLock::new(HashMap::<u64, EventCallback>::new()));
        let subs_for_read = subscriptions.clone();

        // Spawn write task
        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    WsCommand::SendMessage { message } => {
                        if write.send(Message::Text(message.into())).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        // Spawn read task
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Parse and dispatch events
                        if let Ok(notification) = serde_json::from_str::<Notification>(&text) {
                            let subs = subs_for_read.read().await;
                            if let Some(callback) = subs.get(&notification.params.subscription) {
                                // Parse event based on method
                                let event = parse_event(&notification.method, &notification.params.result);
                                callback(event);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        connected.store(false, Ordering::Relaxed);
                        break;
                    }
                    Err(_) => {
                        connected.store(false, Ordering::Relaxed);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Get next request ID.
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Subscribe to account changes.
    pub async fn subscribe_account<F>(
        &self,
        pubkey: &Pubkey,
        callback: F,
    ) -> Result<SubscriptionId>
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let request_id = self.next_id();
        let request = SubscribeRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "accountSubscribe".to_string(),
            params: serde_json::json!([
                pubkey.to_string(),
                {"encoding": "base64", "commitment": "confirmed"}
            ]),
        };

        let sub_id = self.send_subscribe_request(request).await?;

        // Wrap callback
        let wrapped: EventCallback = Box::new(move |event| {
            if let WebsocketEvent::AccountChange { data, .. } = event {
                callback(data);
            }
        });

        self.subscriptions.write().await.insert(
            sub_id,
            Subscription {
                id: sub_id,
                callback: wrapped,
            },
        );

        Ok(sub_id)
    }

    /// Subscribe to program logs.
    pub async fn subscribe_logs<F>(
        &self,
        program_id: &Pubkey,
        callback: F,
    ) -> Result<SubscriptionId>
    where
        F: Fn(Vec<String>) + Send + Sync + 'static,
    {
        let request_id = self.next_id();
        let request = SubscribeRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "logsSubscribe".to_string(),
            params: serde_json::json!([
                {"mentions": [program_id.to_string()]},
                {"commitment": "confirmed"}
            ]),
        };

        let sub_id = self.send_subscribe_request(request).await?;

        let wrapped: EventCallback = Box::new(move |event| {
            if let WebsocketEvent::LogsUpdate { logs, .. } = event {
                callback(logs);
            }
        });

        self.subscriptions.write().await.insert(
            sub_id,
            Subscription {
                id: sub_id,
                callback: wrapped,
            },
        );

        Ok(sub_id)
    }

    /// Subscribe to slot updates.
    pub async fn subscribe_slots<F>(&self, callback: F) -> Result<SubscriptionId>
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        let request_id = self.next_id();
        let request = SubscribeRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "slotSubscribe".to_string(),
            params: serde_json::json!([]),
        };

        let sub_id = self.send_subscribe_request(request).await?;

        let wrapped: EventCallback = Box::new(move |event| {
            if let WebsocketEvent::SlotUpdate(slot) = event {
                callback(slot);
            }
        });

        self.subscriptions.write().await.insert(
            sub_id,
            Subscription {
                id: sub_id,
                callback: wrapped,
            },
        );

        Ok(sub_id)
    }

    /// Wait for transaction confirmation.
    pub async fn wait_for_confirmation(
        &self,
        signature: &Signature,
        timeout_ms: u64,
    ) -> Result<bool> {
        let sig = *signature;
        let (tx, mut rx) = mpsc::channel(1);

        let request_id = self.next_id();
        let request = SubscribeRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "signatureSubscribe".to_string(),
            params: serde_json::json!([
                sig.to_string(),
                {"commitment": "confirmed"}
            ]),
        };

        let sub_id = self.send_subscribe_request(request).await?;

        let wrapped: EventCallback = Box::new(move |event| {
            if let WebsocketEvent::TransactionConfirmed(_) = event {
                let _ = tx.try_send(true);
            }
        });

        self.subscriptions.write().await.insert(
            sub_id,
            Subscription {
                id: sub_id,
                callback: wrapped,
            },
        );

        // Wait for confirmation or timeout
        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            rx.recv(),
        )
        .await;

        // Cleanup subscription
        self.unsubscribe(sub_id).await?;

        match result {
            Ok(Some(true)) => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Err(WsError::Timeout("Transaction confirmation timeout".to_string()).into()),
        }
    }

    /// Unsubscribe.
    pub async fn unsubscribe(&self, id: SubscriptionId) -> Result<()> {
        self.subscriptions.write().await.remove(&id);

        // Send unsubscribe request (simplified - real impl would track subscription type)
        let request_id = self.next_id();
        let request = SubscribeRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "slotUnsubscribe".to_string(), // Would need to track actual method
            params: serde_json::json!([id]),
        };

        if let Some(tx) = &self.command_tx {
            let msg = serde_json::to_string(&request)
                .map_err(|e| WsError::ParseError(e.to_string()))?;
            tx.send(WsCommand::SendMessage { message: msg })
                .await
                .map_err(|e| WsError::SendFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// Send subscription request.
    async fn send_subscribe_request(&self, request: SubscribeRequest) -> Result<u64> {
        let tx = self
            .command_tx
            .as_ref()
            .ok_or_else(|| WsError::ConnectionFailed("Not connected".to_string()))?;

        let msg = serde_json::to_string(&request)
            .map_err(|e| WsError::ParseError(e.to_string()))?;

        tx.send(WsCommand::SendMessage { message: msg })
            .await
            .map_err(|e| WsError::SendFailed(e.to_string()))?;

        // In a real implementation, we'd wait for the subscription response
        // For now, return the request ID as subscription ID
        Ok(request.id)
    }

    /// Close the connection.
    pub async fn close(&mut self) {
        self.command_tx = None;
        self.connected.store(false, Ordering::Relaxed);
    }
}

/// Parse websocket event from notification.
fn parse_event(method: &str, result: &serde_json::Value) -> WebsocketEvent {
    match method {
        "slotNotification" => {
            let slot = result.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            WebsocketEvent::SlotUpdate(slot)
        }
        "accountNotification" => {
            let data = result
                .get("value")
                .and_then(|v| v.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|s| s.as_str())
                .and_then(|s| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok())
                .unwrap_or_default();

            let slot = result
                .get("context")
                .and_then(|c| c.get("slot"))
                .and_then(|s| s.as_u64())
                .unwrap_or(0);

            WebsocketEvent::AccountChange {
                pubkey: Pubkey::default(), // Would need to track this
                data,
                slot,
            }
        }
        "logsNotification" => {
            let logs = result
                .get("value")
                .and_then(|v| v.get("logs"))
                .and_then(|l| l.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let sig_str = result
                .get("value")
                .and_then(|v| v.get("signature"))
                .and_then(|s| s.as_str())
                .unwrap_or_default();

            let signature = sig_str.parse().unwrap_or_default();

            WebsocketEvent::LogsUpdate { signature, logs }
        }
        "signatureNotification" => {
            let sig_str = result
                .get("value")
                .and_then(|v| v.get("signature"))
                .and_then(|s| s.as_str())
                .unwrap_or_default();

            let signature = sig_str.parse().unwrap_or_default();
            WebsocketEvent::TransactionConfirmed(signature)
        }
        _ => WebsocketEvent::Error(format!("Unknown method: {}", method)),
    }
}

/// Pool watcher for detecting new liquidity pools.
pub struct PoolWatcher {
    ws: SolanaWebsocket,
    dex_programs: Vec<Pubkey>,
}

impl PoolWatcher {
    /// Create a new pool watcher.
    pub fn new(wss_url: &str) -> Self {
        Self {
            ws: SolanaWebsocket::new(wss_url),
            dex_programs: vec![],
        }
    }

    /// Add DEX program to watch.
    pub fn watch_dex(&mut self, program_id: Pubkey) {
        self.dex_programs.push(program_id);
    }

    /// Start watching for new pools.
    pub async fn start<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Pubkey, Vec<String>) + Send + Sync + Clone + 'static,
    {
        self.ws.connect().await?;

        for program_id in &self.dex_programs {
            let cb = callback.clone();
            let pid = *program_id;

            self.ws
                .subscribe_logs(program_id, move |logs| {
                    // Check for pool creation logs
                    let is_pool_creation = logs.iter().any(|log| {
                        log.contains("InitializePool")
                            || log.contains("Initialize2")
                            || log.contains("createPool")
                    });

                    if is_pool_creation {
                        cb(pid, logs);
                    }
                })
                .await?;
        }

        Ok(())
    }

    /// Stop watching.
    pub async fn stop(&mut self) {
        self.ws.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_creation() {
        let ws = SolanaWebsocket::new("wss://api.mainnet-beta.solana.com");
        assert!(!ws.is_connected());
    }

    #[test]
    fn test_parse_slot_event() {
        let result = serde_json::json!({"slot": 12345});
        let event = parse_event("slotNotification", &result);

        match event {
            WebsocketEvent::SlotUpdate(slot) => assert_eq!(slot, 12345),
            _ => panic!("Expected SlotUpdate"),
        }
    }

    #[test]
    fn test_parse_logs_event() {
        let result = serde_json::json!({
            "value": {
                "logs": ["Program log: Instruction: Initialize", "Program success"],
                "signature": "5K..."
            }
        });

        let event = parse_event("logsNotification", &result);

        match event {
            WebsocketEvent::LogsUpdate { logs, .. } => {
                assert_eq!(logs.len(), 2);
            }
            _ => panic!("Expected LogsUpdate"),
        }
    }

    #[test]
    fn test_pool_watcher_creation() {
        let mut watcher = PoolWatcher::new("wss://api.mainnet-beta.solana.com");
        watcher.watch_dex(Pubkey::new_unique());
        assert_eq!(watcher.dex_programs.len(), 1);
    }
}
