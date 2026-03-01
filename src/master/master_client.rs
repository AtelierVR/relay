use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use tokio::sync::{oneshot, Notify};
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::Config,
    utils::log_buffer::{LogBuffer, LogEntry},
};

use super::messages::{
    DropInstance, LogsRequest, RelayConnected, RelayStatus, RequestInstancesReq,
    RequestInstancesResp, ResolveUserRequest, ResolveUserResponse, SyncInstancesResp, WsMessage,
};

type PendingMap = Arc<DashMap<String, oneshot::Sender<Value>>>;

/// Async WebSocket client for the MasterServer connection.
///
/// Reconnects automatically with exponential back-off on disconnect.
#[derive(Debug)]
pub struct MasterClient {
    config: Arc<Config>,
    /// Pending correlated requests: message-id → response sender.
    pending: PendingMap,
    /// Notify token: signal the send task that a new message is queued.
    send_notify: Arc<Notify>,
    /// Outbound message queue (JSON strings).
    send_queue: Arc<Mutex<Vec<String>>>,
    /// Buffer of recent log entries for the "logs" query.
    pub log_buf: Arc<parking_lot::Mutex<LogBuffer>>,
    /// Relay start time (Unix ms).
    pub start_time_ms: i64,
}

impl MasterClient {
    pub fn new(config: Arc<Config>) -> Self {
        let start_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Self {
            config,
            pending: Arc::new(DashMap::new()),
            send_notify: Arc::new(Notify::new()),
            send_queue: Arc::new(Mutex::new(Vec::new())),
            log_buf: Arc::new(parking_lot::Mutex::new(LogBuffer::new(1000))),
            start_time_ms,
        }
    }

    // ── Low-level send ───────────────────────────────────────────────────

    /// Queue a JSON message for sending on the current WS connection.
    pub fn emit_raw(&self, json: String) {
        self.send_queue.lock().push(json);
        self.send_notify.notify_one();
    }

    /// Serialize and queue a typed message (fire-and-forget).
    pub fn emit<T: serde::Serialize>(&self, msg_type: &str, data: T) -> Result<()> {
        let msg = WsMessage::new(msg_type, data);
        let json = serde_json::to_string(&msg)?;
        self.emit_raw(json);
        Ok(())
    }

    /// Serialize, queue, and await a response with a correlated UUID.
    pub async fn request<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        msg_type: &str,
        data: T,
        timeout_secs: u64,
    ) -> Result<R> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<Value>();
        self.pending.insert(id.clone(), tx);

        let mut msg = WsMessage::new(msg_type, data);
        msg.id = Some(id.clone());
        let json = serde_json::to_string(&msg)?;
        self.emit_raw(json);

        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await;

        self.pending.remove(&id);

        match result {
            Ok(Ok(val)) => serde_json::from_value(val)
                .map_err(|e| anyhow!("response deserialize: {e}")),
            Ok(Err(_)) => Err(anyhow!("request channel closed")),
            Err(_) => Err(anyhow!("request timed out after {timeout_secs}s")),
        }
    }

    // ── High-level helpers ───────────────────────────────────────────────

    pub async fn resolve_user(
        &self,
        user_id: u32,
        server: String,
        fingerprint: String,
    ) -> Result<ResolveUserResponse> {
        self.request(
            "resolve_user",
            ResolveUserRequest { user_id, server, fingerprint },
            10,
        )
        .await
    }

    pub async fn request_instances(&self, count: u8) -> Result<RequestInstancesResp> {
        self.request("request_instances", RequestInstancesReq { count }, 15).await
    }

    pub fn send_log(&self, timestamp: i64, level: &str, message: &str) {
        let _ = self.emit(
            "log",
            serde_json::json!({
                "Timestamp": timestamp,
                "Level": level,
                "Message": message,
            }),
        );
    }

    // ── Reconnect loop ───────────────────────────────────────────────────

    /// Run the persistent reconnect loop.  Call this in a dedicated `tokio::spawn`.
    pub async fn run(self: Arc<Self>, on_ready: impl Fn() + Send + 'static) {
        let mut backoff = Duration::from_secs(1);
        const MAX_BACKOFF: Duration = Duration::from_secs(30);

        loop {
            let cfg = &self.config;
            let ws_url = Self::build_ws_url(&cfg.master_gateway);

            info!("[MasterClient] Connecting to {ws_url}");

            match self.run_session(&ws_url, &on_ready).await {
                Ok(()) => {
                    warn!("[MasterClient] Session ended cleanly; reconnecting…");
                }
                Err(e) => {
                    warn!("[MasterClient] Session error: {e}; reconnecting in {backoff:?}");
                }
            }

            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }

    async fn run_session(
        self: &Arc<Self>,
        url: &str,
        on_ready: &impl Fn(),
    ) -> Result<()> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let mut request = url.into_client_request()?;
        let token = &self.config.token;
        if !token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                format!("Badger {token}").parse()?,
            );
        }

        let (ws_stream, _) = connect_async_tls_with_config(request, None, false, None).await?;
        info!("[MasterClient] WebSocket connected");

        // Reset back-off on success is handled by the caller resetting after Ok return.
        let (mut sink, mut stream) = ws_stream.split();

        on_ready();

        let pending = Arc::clone(&self.pending);
        let send_queue = Arc::clone(&self.send_queue);
        let send_notify = Arc::clone(&self.send_notify);

        // Outbound task: drain the queue whenever notified.
        let send_task = tokio::spawn(async move {
            loop {
                send_notify.notified().await;
                let msgs: Vec<String> = {
                    let mut q = send_queue.lock();
                    std::mem::take(&mut *q)
                };
                for json in msgs {
                    if sink.send(Message::Text(json)).await.is_err() {
                        return;
                    }
                }
            }
        });

        // Inbound loop.
        while let Some(msg) = stream.next().await {
            match msg? {
                Message::Text(text) => {
                    self.handle_inbound(&text).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        send_task.abort();
        Ok(())
    }

    /// Process one inbound WS text message.
    async fn handle_inbound(&self, text: &str) {
        let Ok(envelope) = serde_json::from_str::<WsMessage<Value>>(text) else {
            warn!("[MasterClient] Failed to parse inbound: {text}");
            return;
        };

        // Correlated response?
        if let Some(id) = &envelope.id {
            if let Some((_, tx)) = self.pending.remove(id) {
                let _ = tx.send(envelope.data);
                return;
            }
        }

        // Server-initiated messages.
        match envelope.msg_type.as_str() {
            "relay_connected" => {
                if let Ok(rc) = serde_json::from_value::<RelayConnected>(envelope.data) {
                    info!("[MasterClient] relay_connected id={} master_address={}", rc.id, rc.master_address);
                }
            }
            "drop_instance" => {
                if let Ok(di) = serde_json::from_value::<DropInstance>(envelope.data) {
                    warn!("[MasterClient] drop_instance #{}: {}", di.instance_id, di.message.unwrap_or_default());
                    // Actual instance removal is handled by a registered callback; see AppState.
                }
            }
            "status" => {
                // Status request — the response is sent by AppState logic; nothing to do here.
                debug!("[MasterClient] status request (id={:?})", envelope.id);
            }
            "logs" => {
                if let Ok(req) = serde_json::from_value::<LogsRequest>(envelope.data.clone()) {
                    let entries = {
                        let buf = self.log_buf.lock();
                        if req.since > 0 {
                            buf.since(req.since)
                        } else {
                            buf.last_n(req.limit)
                        }
                    };
                    let _ = self.emit(
                        "logs",
                        serde_json::json!({ "logs": entries }),
                    );
                }
            }
            other => {
                debug!("[MasterClient] unhandled inbound type: {other}");
            }
        }
    }

    // ── Utilities ────────────────────────────────────────────────────────

    fn build_ws_url(master_gateway: &str) -> String {
        let clean = master_gateway.trim_end_matches('/');
        if clean.starts_with("https://") {
            format!("wss://{}/api/ws", &clean[8..])
        } else if clean.starts_with("http://") {
            format!("ws://{}/api/ws", &clean[7..])
        } else {
            format!("ws://{clean}/api/ws")
        }
    }
}
