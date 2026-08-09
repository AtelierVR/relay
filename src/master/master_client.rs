use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use tokio::sync::{oneshot, Notify};
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::utils::system_specs::get_specs;

use crate::{
    client::ClientManager, config::Config, instance::InstanceManager, utils::log_buffer::LogBuffer,
    utils::log_layer::LogForwarder,
};

use super::{
    messages::{
        ClientInfo, CommandRequest, DropInstance, GetClientsReq, GetClientsResp, GetInstancesReq,
        GetInstancesResp, GetPlayersReq, GetPlayersResp, HasInstanceReq, HasInstanceResp,
        InstanceInfo, LogsRequest, PingResponse, PlayerInfo, RelayConnected, RequestInstancesReq,
        RequestInstancesResp, ResolveUserRequest, ResolveUserResponse, SyncInstanceData,
        SyncInstancesReq, SyncInstancesResp, WorldInfo, WsMessage,
    },
    relay_extensions,
};

type PendingMap = Arc<DashMap<String, oneshot::Sender<Value>>>;

/// Async WebSocket client for the MasterServer connection.
///
/// Reconnects automatically with exponential back-off on disconnect.
#[derive(Debug)]
pub struct MasterClient {
    config: Arc<Config>,
    clients: Arc<ClientManager>,
    instances: Arc<InstanceManager>,
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
    /// Whether the WS connection is currently alive (for background tasks).
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// Real-time log forwarder: set during an active WS session.
    log_forwarder: LogForwarder,
    /// Reference to AppState for command execution (set after construction).
    pub state: parking_lot::Mutex<Option<Arc<crate::handlers::context::AppState>>>,
}

impl MasterClient {
    pub fn new(
        config: Arc<Config>,
        clients: Arc<ClientManager>,
        instances: Arc<InstanceManager>,
        log_buf: Arc<parking_lot::Mutex<LogBuffer>>,
        log_forwarder: LogForwarder,
    ) -> Self {
        let start_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Self {
            config,
            clients,
            instances,
            pending: Arc::new(DashMap::new()),
            send_notify: Arc::new(Notify::new()),
            send_queue: Arc::new(Mutex::new(Vec::new())),
            log_buf,
            start_time_ms,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            log_forwarder,
            state: parking_lot::Mutex::new(None),
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
            Ok(Ok(val)) => {
                serde_json::from_value(val).map_err(|e| anyhow!("response deserialize: {e}"))
            }
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
            ResolveUserRequest {
                user_id,
                server,
                fingerprint,
            },
            10,
        )
        .await
    }

    pub async fn request_instances(&self, count: u8) -> Result<RequestInstancesResp> {
        self.request("request_instances", RequestInstancesReq { count }, 15)
            .await
    }

    /// Sync existing instances with the master server.
    pub async fn sync_instances_with_master(&self) -> Result<SyncInstancesResp> {
        let relay_uptime = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
            - self.start_time_ms) as f64
            / 1000.0;

        let instances: Vec<SyncInstanceData> = self
            .instances
            .all()
            .iter()
            .map(|arc_inst| {
                let inst = arc_inst.read();
                SyncInstanceData {
                    master_id: inst.master_id,
                    internal_id: inst.internal_id as u32,
                    password: None, // Password hash is not synced back
                    capacity: inst.capacity,
                    player_count: inst.get_players().len(),
                    world: inst.world.to_identifier(),
                    flags: format!("{:?}", inst.flags),
                }
            })
            .collect();

        let req = SyncInstancesReq {
            instances,
            relay_uptime,
        };

        self.request("relay_sync_instances", req, 10).await
    }

    /// Called when connected to master. Syncs existing instances and requests new ones if needed.
    pub async fn on_connected(&self) {
        use crate::instance::{Instance, InstanceFlags, World};

        // Small delay to ensure send_task is ready before sending messages
        // This prevents race condition where notify_one() is called before notified().await
        // tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        info!("[MasterClient] on_connected() started");

        let max_instances = self.config.max_instances;
        let current_count = self.instances.count();

        info!(
            "[MasterClient] Connected. Current instances: {}/{}",
            current_count, max_instances
        );

        // Sync existing instances if any
        if current_count > 0 {
            debug!(
                "[MasterClient] Found {} existing instances, syncing with master...",
                current_count
            );

            match self.sync_instances_with_master().await {
                Ok(resp) => {
                    if resp.success {
                        debug!(
                            "[MasterClient] Successfully synced {} instances (conflicts: {})",
                            resp.synced_count, resp.conflicts_resolved
                        );

                        // Remove invalid instances reported by master
                        if let Some(invalid) = resp.invalid_instances {
                            if !invalid.is_empty() {
                                warn!(
                                    "[MasterClient] Master reported {} invalid instances, removing...",
                                    invalid.len()
                                );

                                for master_id in invalid {
                                    // Find and remove instance with this master_id
                                    let mut to_remove = None;
                                    self.instances.for_each(|internal_id, arc_inst| {
                                        if arc_inst.read().master_id == master_id {
                                            to_remove = Some(internal_id);
                                        }
                                    });

                                    if let Some(id) = to_remove {
                                        self.instances.remove(id);
                                        debug!(
                                            "[MasterClient] Removed invalid instance #{}",
                                            master_id
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        error!(
                            "[MasterClient] Master rejected instance sync: {}",
                            resp.error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                        return;
                    }
                }
                Err(e) => {
                    warn!("[MasterClient] Failed to sync instances: {}", e);
                    return;
                }
            }
        }

        // Calculate how many additional instances we need
        let current_count = self.instances.count(); // May have changed after sync
        info!(
            "[MasterClient] Re-counted instances: current_count={}, max_instances={}",
            current_count, max_instances
        );
        let needed = max_instances.saturating_sub(current_count as u8);
        info!("[MasterClient] Calculated needed instances: {}", needed);

        if needed == 0 {
            debug!(
                "[MasterClient] Have {}/{} instances, no additional instances needed",
                current_count, max_instances
            );
            return;
        }

        // Request additional instances
        debug!(
            "[MasterClient] Requesting {} additional instances (currently have {}/{})",
            needed, current_count, max_instances
        );

        debug!("[MasterClient] Calling request_instances({})...", needed);
        match self.request_instances(needed).await {
            Ok(resp) => {
                if !resp.success {
                    error!(
                        "[MasterClient] Failed to request instances: {}",
                        resp.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                    return;
                }

                debug!("[MasterClient] request_instances response success=true");
                let instances = resp.instances;
                let num_instances = instances.len();
                info!(
                    "[MasterClient] Received {} instances from master server",
                    num_instances
                );

                if num_instances == 0 {
                    warn!(
                        "[MasterClient] Master returned 0 instances (requested: {})",
                        needed
                    );
                }

                // Create instances locally
                for spec in instances {
                    // Skip if we already have a slot for this DB instance (avoids duplicates
                    // across reconnections when the node returns the same instance again).
                    if self.instances.has_master_id(spec.id) {
                        debug!(
                            "[MasterClient] Instance #{} already exists, skipping",
                            spec.id
                        );
                        continue;
                    }

                    let internal_id = self.instances.next_internal_id();
                    if internal_id == u8::MAX {
                        warn!(
                            "[MasterClient] No available internal IDs, stopping instance creation"
                        );
                        break;
                    }

                    let app_state = self.state.lock().clone();
                    let mut instance = Instance::new(internal_id, spec.id, app_state);
                    instance.capacity = spec.capacity;
                    instance.property_resend_interval = self.config.property_resend_interval;

                    // In debug mode, authorize bots by default
                    if self.config.debug {
                        instance.flags.insert(InstanceFlags::AUTHORIZE_BOT);
                    }

                    if let Some(pwd) = spec.password {
                        instance.set_password(Some(pwd));
                    }

                    if let Some(world_spec) = spec.world {
                        instance.world =
                            World::new(world_spec.id, world_spec.address, world_spec.version);
                    }

                    self.instances.add(instance);
                    debug!(
                        "[MasterClient] Created instance #{} (internal ID: {}) - Total instances now: {}",
                        spec.id, internal_id, self.instances.count()
                    );
                }

                // Sync newly created instances with master
                if num_instances > 0 {
                    debug!("[MasterClient] Syncing newly received instances with master...");
                    match self.sync_instances_with_master().await {
                        Ok(_) => {
                            debug!("[MasterClient] Successfully synced new instances");
                        }
                        Err(e) => {
                            warn!("[MasterClient] Failed to sync new instances: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("[MasterClient] Error requesting instances: {}", e);
            }
        }
    }

    pub fn send_log(&self, timestamp: i64, level: &str, message: &str, tag: Option<&str>) {
        let mut data = serde_json::json!({
            "a": timestamp,
            "l": level,
            "m": message
        });
        if let Some(t) = tag {
            data["t"] = serde_json::json!(t);
        }
        let _ = self.emit("log", data);
    }

    fn handle_command(&self, content: &str) {
        if let Some(ref state) = *self.state.lock() {
            if let Some(ref registry) = *state.commands.lock().unwrap() {
                registry.execute(content);
            }
        }
    }

    // ── Reconnect loop ───────────────────────────────────────────────────

    /// Run the persistent reconnect loop.  Call this in a dedicated `tokio::spawn`.
    pub async fn run(self: Arc<Self>) {
        let mut backoff = Duration::from_secs(1);
        const MAX_BACKOFF: Duration = Duration::from_secs(30);

        loop {
            let cfg = &self.config;
            let ws_url = Self::build_ws_url(&cfg.node_gateway);

            info!("[MasterClient] Connecting to {ws_url}");

            match self.run_session(&ws_url).await {
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

    async fn run_session(self: &Arc<Self>, url: &str) -> Result<()> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let mut request = url.into_client_request()?;
        let token = &self.config.token;
        if !token.is_empty() {
            request
                .headers_mut()
                .insert("Authorization", format!("Badger {token}").parse()?);
        }

        let (ws_stream, _) = connect_async_tls_with_config(request, None, false, None).await?;
        info!("[MasterClient] WebSocket connected");

        // Reset back-off on success is handled by the caller resetting after Ok return.
        let (mut sink, mut stream) = ws_stream.split();

        self.connected.store(true, Ordering::Relaxed);

        let send_queue = Arc::clone(&self.send_queue);
        let send_notify = Arc::clone(&self.send_notify);
        let connected = Arc::clone(&self.connected);
        let instances_p = Arc::clone(&self.instances);

        // ── Real-time log forwarding task ─────────────────────────────────
        let (log_tx, mut log_rx) =
            tokio::sync::mpsc::unbounded_channel::<crate::utils::log_buffer::LogEntry>();
        *self.log_forwarder.lock() = Some(log_tx);
        let log_sq = Arc::clone(&self.send_queue);
        let log_sn = Arc::clone(&self.send_notify);
        let log_conn = Arc::clone(&connected);
        let log_task = tokio::spawn(async move {
            while let Some(entry) = log_rx.recv().await {
                if !log_conn.load(Ordering::Relaxed) {
                    break;
                }
                let mut data = serde_json::json!({
                    "a": entry.timestamp,
                    "l": entry.level,
                    "m": entry.message,
                });
                if let Some(tag) = entry.tag {
                    data["t"] = serde_json::json!(tag);
                }
                let msg = WsMessage::new("log", data);
                if let Ok(json) = serde_json::to_string(&msg) {
                    log_sq.lock().push(json);
                    log_sn.notify_one();
                }
            }
        });

        // ── Ping task (every 15 s) ────────────────────────────────────────
        let ping_sq = Arc::clone(&self.send_queue);
        let ping_sn = Arc::clone(&self.send_notify);
        let ping_conn = Arc::clone(&connected);
        let ping_inst = Arc::clone(&instances_p);
        let ping_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;
                if !ping_conn.load(Ordering::Relaxed) {
                    break;
                }
                let count = ping_inst.all().len() as u32;
                let msg = WsMessage::new(
                    "ping",
                    serde_json::json!({
                        "time": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                        "instance_count": count
                    }),
                );
                if let Ok(json) = serde_json::to_string(&msg) {
                    ping_sq.lock().push(json);
                    ping_sn.notify_one();
                }
            }
        });

        // ── Specs push task (every 500 ms) ────────────────────────────────
        let specs_sq = Arc::clone(&self.send_queue);
        let specs_sn = Arc::clone(&self.send_notify);
        let specs_conn = Arc::clone(&connected);
        let specs_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !specs_conn.load(Ordering::Relaxed) {
                    break;
                }
                // get_specs() performs blocking I/O; run it off the async executor.
                let specs = tokio::task::spawn_blocking(get_specs).await;
                let specs = match specs {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("[MasterClient] specs task error: {e}");
                        continue;
                    }
                };
                let msg = WsMessage::new("specs", &specs);
                if let Ok(json) = serde_json::to_string(&msg) {
                    specs_sq.lock().push(json);
                    specs_sn.notify_one();
                }
            }
        });

        // ── Outbound task: drain the queue whenever notified ──────────────
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

        // ── Initialize instances after connection established ──────────────
        {
            let self_ref = Arc::clone(self);
            tokio::spawn(async move {
                self_ref.on_connected().await;
            });
        }

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

        self.connected.store(false, Ordering::Relaxed);
        *self.log_forwarder.lock() = None;
        log_task.abort();
        send_task.abort();
        ping_task.abort();
        specs_task.abort();
        Ok(())
    }

    /// Process one inbound WS text message.
    async fn handle_inbound(&self, text: &str) {
        use super::messages::InstanceSpec;
        use crate::instance::{Instance, InstanceFlags, World};
        let Ok(envelope) = serde_json::from_str::<WsMessage<Value>>(text) else {
            warn!("[MasterClient] Failed to parse inbound: {text}");
            return;
        };

        // Correlated response?
        if let Some(id) = &envelope.id {
            if let Some((_, tx)) = self.pending.remove(id) {
                let _ = tx.send(envelope.payload);
                return;
            }
        }

        // Server-initiated messages.
        match envelope.msg_type.as_str() {
            "relay_connected" => {
                if let Ok(rc) = serde_json::from_value::<RelayConnected>(envelope.payload) {
                    info!(
                        "[MasterClient] relay_connected id={} master_address={}",
                        rc.id, rc.master_address
                    );
                }
            }
            "new_instance" => {
                if let Ok(spec) = serde_json::from_value::<InstanceSpec>(envelope.payload) {
                    let current = self.instances.count();
                    let max = self.config.max_instances as usize;
                    if current >= max {
                        warn!(
                            "[MasterClient] new_instance: at capacity ({}/{}), ignoring",
                            current, max
                        );
                        return;
                    }
                    let internal_id = self.instances.next_internal_id();
                    if internal_id == u8::MAX {
                        warn!("[MasterClient] new_instance: no available internal IDs");
                        return;
                    }
                    let app_state2 = self.state.lock().clone();
                    let mut instance = Instance::new(internal_id, spec.id, app_state2);
                    instance.capacity = spec.capacity;
                    instance.property_resend_interval = self.config.property_resend_interval;
                    if self.config.debug {
                        instance.flags.insert(InstanceFlags::AUTHORIZE_BOT);
                    }
                    if let Some(pwd) = spec.password {
                        instance.set_password(Some(pwd));
                    }
                    if let Some(world_spec) = spec.world {
                        instance.world =
                            World::new(world_spec.id, world_spec.address, world_spec.version);
                    }
                    self.instances.add(instance);
                    info!(
                        "[MasterClient] new_instance: created instance #{} (internal {})",
                        spec.id, internal_id
                    );
                    if let Err(e) = self.sync_instances_with_master().await {
                        warn!("[MasterClient] new_instance: sync failed: {}", e);
                    }
                }
            }
            "drop_instance" => {
                if let Ok(di) = serde_json::from_value::<DropInstance>(envelope.payload) {
                    warn!(
                        "[MasterClient] drop_instance #{}: {}",
                        di.instance_id,
                        di.message.as_deref().unwrap_or_default()
                    );
                    let internal_id = self
                        .instances
                        .all()
                        .into_iter()
                        .find(|arc| arc.read().master_id == di.instance_id)
                        .map(|arc| arc.read().internal_id);
                    if let Some(id) = internal_id {
                        self.instances.remove(id);
                        debug!(
                            "[MasterClient] instance #{} (internal {id}) dropped",
                            di.instance_id
                        );
                    } else {
                        warn!(
                            "[MasterClient] drop_instance: instance #{} not found locally",
                            di.instance_id
                        );
                    }
                }
            }
            "status" => {
                let status = relay_extensions::build_status(
                    &self.clients,
                    &self.instances,
                    self.config.max_instances,
                    self.start_time_ms,
                    &self.config,
                );
                let mut msg = WsMessage::new("status", status);
                if let Some(id) = envelope.id {
                    msg = msg.with_id(id);
                }
                if let Ok(json) = serde_json::to_string(&msg) {
                    self.emit_raw(json);
                }
            }
            "logs" => {
                if let Ok(req) = serde_json::from_value::<LogsRequest>(envelope.payload.clone()) {
                    let entries = {
                        let buf = self.log_buf.lock();
                        if req.since > 0 {
                            buf.since(req.since)
                        } else {
                            buf.last_n(req.limit)
                        }
                    };
                    let mut msg = WsMessage::new("logs", serde_json::json!({ "logs": entries }));
                    if let Some(id) = envelope.id {
                        msg = msg.with_id(id);
                    }
                    if let Ok(json) = serde_json::to_string(&msg) {
                        self.emit_raw(json);
                    }
                }
            }
            "has_instance" => {
                if let Ok(req) = serde_json::from_value::<HasInstanceReq>(envelope.payload.clone())
                {
                    let exists = self
                        .instances
                        .all()
                        .iter()
                        .any(|arc| arc.read().master_id == req.id);
                    let resp = HasInstanceResp { exists };
                    let mut msg = WsMessage::new("has_instance", resp);
                    if let Some(id) = envelope.id {
                        msg = msg.with_id(id);
                    }
                    if let Ok(json) = serde_json::to_string(&msg) {
                        self.emit_raw(json);
                    }
                }
            }
            "get_clients" => {
                if let Ok(req) = serde_json::from_value::<GetClientsReq>(envelope.payload.clone()) {
                    let total = self.clients.count() as u32;
                    let mut clients: Vec<ClientInfo> = Vec::new();
                    let mut count = 0;
                    let mut skipped = 0;
                    let limit = if req.limit > 0 { req.limit } else { 100 };

                    self.clients.for_each(|_, arc| {
                        if skipped < req.offset {
                            skipped += 1;
                            return;
                        }
                        if count >= limit {
                            return;
                        }
                        let c = arc.read();
                        clients.push(ClientInfo {
                            id: c.id,
                            address: c.address.clone(),
                            platform: c.platform.clone(),
                            engine: c.engine.clone(),
                            user: c.user.as_ref().map(|u| u.to_identifier()),
                            connected_at: c.connected_at,
                        });
                        count += 1;
                    });

                    let resp = GetClientsResp { total, clients };
                    let mut msg = WsMessage::new("get_clients", resp);
                    if let Some(id) = envelope.id {
                        msg = msg.with_id(id);
                    }
                    if let Ok(json) = serde_json::to_string(&msg) {
                        self.emit_raw(json);
                    }
                }
            }
            "get_instances" => {
                if let Ok(req) = serde_json::from_value::<GetInstancesReq>(envelope.payload.clone())
                {
                    let all_instances = self.instances.all();
                    let total = all_instances.len() as u32;
                    let instances: Vec<InstanceInfo> = all_instances
                        .iter()
                        .skip(req.offset)
                        .take(if req.limit > 0 { req.limit } else { 100 })
                        .map(|arc| {
                            let inst = arc.read();
                            let (effective_tps, effective_threshold) =
                                inst.get_effective_settings(&self.config.load_balancing);
                            InstanceInfo {
                                internal_id: inst.internal_id as u32,
                                node_id: inst.master_id,
                                flags: inst.flags.bits(),
                                player_count: inst.player_count() as u32,
                                world: WorldInfo {
                                    master_id: inst.world.master_id,
                                    server: inst.world.address.clone(),
                                    version: inst.world.version,
                                },
                                capacity: inst.capacity,
                                tps: inst.tps,
                                threshold: inst.threshold,
                                effective_tps,
                                effective_threshold,
                            }
                        })
                        .collect();
                    let resp = GetInstancesResp { total, instances };
                    let mut msg = WsMessage::new("get_instances", resp);
                    if let Some(id) = envelope.id {
                        msg = msg.with_id(id);
                    }
                    if let Ok(json) = serde_json::to_string(&msg) {
                        self.emit_raw(json);
                    }
                }
            }
            "get_players" => {
                if let Ok(req) = serde_json::from_value::<GetPlayersReq>(envelope.payload.clone()) {
                    use crate::player::PlayerFlags;
                    let all_instances = self.instances.all();
                    let target = all_instances
                        .iter()
                        .find(|arc| arc.read().internal_id as u32 == req.internal_id);
                    let (total, page) = if let Some(arc) = target {
                        let inst = arc.read();
                        let visible: Vec<&crate::player::Player> = inst
                            .get_players()
                            .iter()
                            .filter(|p| req.all || !p.flags.contains(PlayerFlags::HIDE_IN_LIST))
                            .collect();
                        let t = visible.len() as u32;
                        let page: Vec<PlayerInfo> = visible
                            .iter()
                            .skip(req.offset)
                            .take(if req.limit > 0 { req.limit } else { 20 })
                            .map(|p| {
                                let client_arc = self.clients.get(p.client_id);
                                let (display, user_id) = if let Some(arc) = client_arc {
                                    let c = arc.read();
                                    let disp = p.display.clone().unwrap_or_else(|| {
                                        c.user
                                            .as_ref()
                                            .map(|u| u.display_name.clone())
                                            .unwrap_or_else(|| "Unknown".to_string())
                                    });
                                    let uid = c.user.as_ref().map(|u| u.to_identifier());
                                    (disp, uid)
                                } else {
                                    (
                                        p.display.clone().unwrap_or_else(|| "Unknown".to_string()),
                                        None,
                                    )
                                };
                                PlayerInfo {
                                    id: p.id,
                                    client_id: p.client_id,
                                    display,
                                    flags: p.flags.bits(),
                                    user: user_id,
                                    joined_at: p.created_at,
                                    custom_tps: p.custom_tps,
                                    custom_threshold: p.custom_threshold,
                                }
                            })
                            .collect();
                        (t, page)
                    } else {
                        (0, vec![])
                    };
                    let resp = GetPlayersResp {
                        total: total,
                        players: page,
                    };
                    let mut msg = WsMessage::new("get_players", resp);
                    if let Some(id) = envelope.id {
                        msg = msg.with_id(id);
                    }
                    if let Ok(json) = serde_json::to_string(&msg) {
                        self.emit_raw(json);
                    }
                }
            }
            "ping" => {
                if let Ok(_resp) = serde_json::from_value::<PingResponse>(envelope.payload.clone())
                {
                    // Silently handle ping response - latency could be calculated if needed
                }
            }
            "command" => {
                if let Ok(req) = serde_json::from_value::<CommandRequest>(envelope.payload.clone())
                {
                    info!("[Command] $ {}", req.content);
                    self.handle_command(&req.content);
                }
            }
            other => {
                debug!("[MasterClient] unhandled inbound type: {other}");
            }
        }
    }

    // ── Utilities ────────────────────────────────────────────────────────

    fn build_ws_url(node_gateway: &str) -> String {
        let clean = node_gateway.trim_end_matches('/');
        if let Some(stripped) = clean.strip_prefix("https://") {
            format!("wss://{}/api/ws", stripped)
        } else if let Some(stripped) = clean.strip_prefix("http://") {
            format!("ws://{}/api/ws", stripped)
        } else {
            format!("ws://{clean}/api/ws")
        }
    }
}
