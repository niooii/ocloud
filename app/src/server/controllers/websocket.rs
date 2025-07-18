use std::{collections::HashMap, sync::Arc, future::Future, pin::Pin, sync::atomic::{AtomicU64, Ordering}};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;
use enum_dispatch::enum_dispatch;

use crate::server::{controllers::files::FileController, error::{ServerError, ServerResult}, models::files::VirtualPath, ServerState};

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingWebSocketPayload<T: Serialize> {
    /// Event data
    pub d: T,
    /// Sequence number for this connection
    pub s: u64,
    /// Event name (struct name)
    pub t: String,
}

#[derive(Debug, Clone, Serialize, ocloud_macros::WsOutEvent)]
pub struct ErrorEvent {
    pub message: String,
}

pub trait WsOutEvent: Serialize + Clone + 'static { 
    /// Returns the canonical name for this event type.
    /// This is automatically implemented by the #[WsIncomingEvent] macro.
    fn event_name() -> &'static str;
}

#[enum_dispatch]
pub trait WsIncomingEvent {
    async fn handle(self, state: &ServerState, connection_id: Uuid) -> ServerResult<()>;
}

// includes the auto generated enum in build script
include!(concat!(env!("OUT_DIR"), "/ws_events.rs"));

#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub id: Uuid,
    // sends JSON strings
    pub sender: tokio::sync::mpsc::UnboundedSender<String>,
    pub sequence: Arc<AtomicU64>,
}

pub type WebSocketController = Arc<WebSocketControllerInner>;

pub struct WebSocketControllerInner {
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnection>>>,
}

impl Clone for WebSocketControllerInner {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
        }
    }
}

#[ocloud_macros::WsIncomingEvent]
pub struct Ping;

#[derive(ocloud_macros::WsOutEvent, Clone, Serialize)]
pub struct Pong {
    message: String
}

impl WsIncomingEvent for Ping {
    async fn handle(self, state: &ServerState, connection_id: Uuid) -> ServerResult<()> {
        state.ws_controller.send(connection_id, Pong {
            message: "Pong!".into()
        }).await;

        Ok(())
    }
}

impl WebSocketControllerInner {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, websocket: WebSocket, server_state: &ServerState) -> ServerResult<()> {
        let connection_id = Uuid::new_v4();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let server_state_clone = server_state.clone();
        
        let connection = WebSocketConnection {
            id: connection_id,
            sender,
            sequence: Arc::new(AtomicU64::new(0)),
        };

        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, connection);
        }

        info!("WebSocket connection established: {}", connection_id);

        let (mut ws_sender, mut ws_receiver) = websocket.split();
        let connections_clone = Arc::clone(&self.connections);
        let self_clone = self.clone();

        let recv_task = tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("Received WebSocket message: {}", text);
                        
                        // Handle the incoming event
                        if let Err(error) = self_clone.handle_incoming_message(&text, connection_id, &server_state_clone).await {
                            error!("Error handling WebSocket message: {}", error);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed by client: {}", connection_id);
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Clean up connection
            let mut connections = connections_clone.write().await;
            connections.remove(&connection_id);
            info!("WebSocket connection removed: {}", connection_id);
        });

        let send_task = tokio::spawn(async move {
            while let Some(message_json) = receiver.recv().await {
                let message = Message::Text(message_json);
                if let Err(e) = ws_sender.send(message).await {
                    error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
        });

        // Wait for either task to complete (connection closed)
        tokio::select! {
            _ = recv_task => {},
            _ = send_task => {},
        }

        Ok(())
    }


    pub async fn get_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    pub async fn handle_incoming_message(
        &self,
        message: &str,
        connection_id: Uuid,
        server_state: &ServerState,
    ) ->ServerResult<()> {
        // Parse JSON directly into the enum using serde tagged enum deserialization
        let event: WsIncomingEventType = serde_json::from_str(message)
            .map_err(|e| ServerError::ValidationError {field: e.to_string()})?;
        
        // Handle the event using enum dispatch
        event.handle(server_state, connection_id).await
    }


    /// Send function that takes any WsOutEvent and wraps it in OutgoingWebSocketPayload
    pub async fn send<T: WsOutEvent>(&self, connection_id: Uuid, data: T) {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&connection_id) {
            let sequence = connection.sequence.fetch_add(1, Ordering::SeqCst);
            
            let payload = OutgoingWebSocketPayload {
                d: data,
                s: sequence,
                t: T::event_name().to_string(),
            };
            
            let message = match serde_json::to_string(&payload) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize payload: {}", e);
                    return;
                }
            };
            
            if let Err(e) = connection.sender.send(message) {
                error!("Failed to send message to connection {}: {}", connection_id, e);
            }
        }
    }

    /// Broadcast an event to all connections
    pub async fn broadcast<T: WsOutEvent>(&self, data: T) {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            let sequence = connection.sequence.fetch_add(1, Ordering::SeqCst);
            
            let payload = OutgoingWebSocketPayload {
                d: data.clone(),
                s: sequence,
                t: T::event_name().to_string(),
            };
            
            let message = match serde_json::to_string(&payload) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize broadcast payload: {}", e);
                    continue;
                }
            };
            
            if let Err(e) = connection.sender.send(message) {
                error!("Failed to send broadcast message to connection {}: {}", connection.id, e);
            }
        }
    }
}