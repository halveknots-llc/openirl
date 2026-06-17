//! OBS automation adapters.
//!
//! The dry-run adapter is used for tests and no-OBS development. The WebSocket
//! adapter implements the OBS WebSocket v5 handshake/request flow while keeping
//! all OBS access behind the [`ObsController`] trait.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use futures_util::{SinkExt, StreamExt};
use openirl_core::{SceneBundle, SceneRole};
use openirl_scene_templates::{ObsSourceTemplate, SceneMaterializationPlan};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{Duration, timeout},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Error as TungsteniteError, Message},
};
use uuid::Uuid;

const OP_HELLO: u64 = 0;
const OP_IDENTIFY: u64 = 1;
const OP_IDENTIFIED: u64 = 2;
const OP_EVENT: u64 = 5;
const OP_REQUEST: u64 = 6;
const OP_REQUEST_RESPONSE: u64 = 7;

/// OBS adapter errors.
#[derive(Debug, Error)]
pub enum ObsError {
    /// Connection failure.
    #[error("OBS connection failed: {0}")]
    Connection(String),
    /// Authentication failure.
    #[error("OBS authentication failed: {0}")]
    Authentication(String),
    /// Requested scene missing.
    #[error("OBS scene not found: {0}")]
    SceneNotFound(String),
    /// Unexpected protocol message.
    #[error("OBS protocol error: {0}")]
    Protocol(String),
    /// JSON serialization/deserialization failure.
    #[error("OBS JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Operation timed out.
    #[error("OBS operation timed out: {0}")]
    Timeout(String),
    /// Operation unsupported in current adapter.
    #[error("OBS operation unsupported: {0}")]
    Unsupported(String),
}

impl From<TungsteniteError> for ObsError {
    fn from(value: TungsteniteError) -> Self {
        Self::Connection(value.to_string())
    }
}

/// OBS stream status snapshot.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObsStatus {
    /// Whether OBS is reachable.
    pub connected: bool,
    /// Whether OBS is streaming.
    pub streaming: bool,
    /// Current scene name if known.
    pub current_scene: Option<String>,
    /// Adapter label.
    pub adapter: String,
}

/// OBS media-source materialization report.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObsSourceMaterializationReport {
    /// Source/input names created during this call.
    pub created: Vec<String>,
    /// Source/input names updated during this call.
    pub updated: Vec<String>,
    /// Sources skipped because materialization was not supported.
    pub skipped: Vec<String>,
    /// Adapter label.
    pub adapter: String,
}

/// OBS controller abstraction.
#[async_trait]
pub trait ObsController: Send + Sync {
    /// Ensures scene bundle exists.
    async fn ensure_scene_bundle(&self, bundle: &SceneBundle) -> Result<(), ObsError>;
    /// Ensures scene bundle plus planned OBS media/browser/image sources exist.
    async fn ensure_scene_materialization(
        &self,
        plan: &SceneMaterializationPlan,
    ) -> Result<(), ObsError> {
        self.ensure_scene_bundle(&plan.scenes).await
    }
    /// Ensures OBS source/input templates exist.
    async fn ensure_source_templates(
        &self,
        templates: &[ObsSourceTemplate],
    ) -> Result<ObsSourceMaterializationReport, ObsError>;
    /// Switches to a role within a bundle.
    async fn switch_scene(&self, bundle: &SceneBundle, role: SceneRole) -> Result<(), ObsError>;
    /// Starts streaming.
    async fn start_streaming(&self) -> Result<(), ObsError>;
    /// Stops streaming.
    async fn stop_streaming(&self) -> Result<(), ObsError>;
    /// Starts OBS recording.
    async fn start_recording(&self) -> Result<(), ObsError>;
    /// Stops OBS recording.
    async fn stop_recording(&self) -> Result<(), ObsError>;
    /// Saves the OBS replay buffer.
    async fn save_replay_buffer(&self) -> Result<(), ObsError>;
    /// Returns current OBS status.
    async fn status(&self) -> Result<ObsStatus, ObsError>;
    /// Returns adapter action log when supported.
    async fn action_log(&self) -> Result<Vec<String>, ObsError> {
        Ok(Vec::new())
    }
}

/// Dry-run controller used for tests, demos, and no-OBS development.
#[derive(Debug, Clone, Default)]
pub struct DryRunObsController {
    actions: Arc<Mutex<Vec<String>>>,
    current_scene: Arc<Mutex<Option<String>>>,
    streaming: Arc<Mutex<bool>>,
}

impl DryRunObsController {
    /// Returns recorded actions.
    pub async fn actions(&self) -> Vec<String> {
        self.actions.lock().await.clone()
    }

    async fn record(&self, action: impl Into<String>) {
        self.actions.lock().await.push(action.into());
    }
}

#[async_trait]
impl ObsController for DryRunObsController {
    async fn ensure_scene_bundle(&self, bundle: &SceneBundle) -> Result<(), ObsError> {
        self.record(format!("ensure_scene_bundle:{}", bundle.name))
            .await;
        Ok(())
    }

    async fn ensure_scene_materialization(
        &self,
        plan: &SceneMaterializationPlan,
    ) -> Result<(), ObsError> {
        self.ensure_scene_bundle(&plan.scenes).await?;
        self.ensure_source_templates(&plan.sources).await?;
        Ok(())
    }

    async fn ensure_source_templates(
        &self,
        templates: &[ObsSourceTemplate],
    ) -> Result<ObsSourceMaterializationReport, ObsError> {
        let mut created = Vec::new();
        for template in templates {
            self.record(format!(
                "ensure_source:{}:{}:{}",
                template.scene_name, template.input_kind, template.input_name
            ))
            .await;
            created.push(template.input_name.clone());
        }
        Ok(ObsSourceMaterializationReport {
            created,
            updated: Vec::new(),
            skipped: Vec::new(),
            adapter: "dry-run".to_string(),
        })
    }

    async fn switch_scene(&self, bundle: &SceneBundle, role: SceneRole) -> Result<(), ObsError> {
        let Some(scene_name) = bundle.scene_name(role) else {
            return Err(ObsError::SceneNotFound(role.to_string()));
        };
        *self.current_scene.lock().await = Some(scene_name.to_string());
        self.record(format!("switch_scene:{scene_name}")).await;
        Ok(())
    }

    async fn start_streaming(&self) -> Result<(), ObsError> {
        *self.streaming.lock().await = true;
        self.record("start_streaming").await;
        Ok(())
    }

    async fn stop_streaming(&self) -> Result<(), ObsError> {
        *self.streaming.lock().await = false;
        self.record("stop_streaming").await;
        Ok(())
    }

    async fn start_recording(&self) -> Result<(), ObsError> {
        self.record("start_recording").await;
        Ok(())
    }

    async fn stop_recording(&self) -> Result<(), ObsError> {
        self.record("stop_recording").await;
        Ok(())
    }

    async fn save_replay_buffer(&self) -> Result<(), ObsError> {
        self.record("save_replay_buffer").await;
        Ok(())
    }

    async fn status(&self) -> Result<ObsStatus, ObsError> {
        Ok(ObsStatus {
            connected: true,
            streaming: *self.streaming.lock().await,
            current_scene: self.current_scene.lock().await.clone(),
            adapter: "dry-run".to_string(),
        })
    }

    async fn action_log(&self) -> Result<Vec<String>, ObsError> {
        Ok(self.actions().await)
    }
}

/// WebSocket adapter configuration for OBS WebSocket v5.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObsWebSocketConfig {
    /// Full WebSocket URL, usually `ws://127.0.0.1:4455`.
    pub url: String,
    /// Optional OBS WebSocket password.
    pub password: Option<String>,
    /// OBS RPC version requested during Identify.
    pub rpc_version: u32,
    /// Per-request timeout in milliseconds.
    pub request_timeout_ms: u64,
}

impl ObsWebSocketConfig {
    /// Builds a localhost/LAN OBS WebSocket URL from host and port.
    #[must_use]
    pub fn from_host_port(host: impl AsRef<str>, port: u16, password: Option<String>) -> Self {
        Self {
            url: format!("ws://{}:{port}", host.as_ref()),
            password,
            rpc_version: 1,
            request_timeout_ms: 3_000,
        }
    }
}

/// Real OBS WebSocket controller.
#[derive(Debug, Clone)]
pub struct ObsWebSocketController {
    config: ObsWebSocketConfig,
}

type ObsSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

impl ObsWebSocketController {
    /// Creates a controller.
    #[must_use]
    pub const fn new(config: ObsWebSocketConfig) -> Self {
        Self { config }
    }

    async fn connect_identified(&self) -> Result<ObsSocket, ObsError> {
        let (mut socket, _) = connect_async(self.config.url.as_str()).await?;
        let hello = read_json(&mut socket).await?;
        if packet_op(&hello)? != OP_HELLO {
            return Err(ObsError::Protocol("expected OBS Hello packet".to_string()));
        }

        let identify = self.identify_payload(&hello)?;
        send_json(&mut socket, &json!({ "op": OP_IDENTIFY, "d": identify })).await?;

        loop {
            let packet = read_json(&mut socket).await?;
            match packet_op(&packet)? {
                OP_IDENTIFIED => return Ok(socket),
                OP_EVENT => continue,
                op => {
                    return Err(ObsError::Protocol(format!(
                        "expected Identified packet, received op {op}"
                    )));
                }
            }
        }
    }

    fn identify_payload(&self, hello: &Value) -> Result<Value, ObsError> {
        let mut payload = json!({ "rpcVersion": self.config.rpc_version });
        let authentication = hello
            .get("d")
            .and_then(|d| d.get("authentication"))
            .filter(|auth| !auth.is_null());

        if let Some(auth) = authentication {
            let challenge = auth
                .get("challenge")
                .and_then(Value::as_str)
                .ok_or_else(|| ObsError::Authentication("missing challenge".to_string()))?;
            let salt = auth
                .get("salt")
                .and_then(Value::as_str)
                .ok_or_else(|| ObsError::Authentication("missing salt".to_string()))?;
            let password = self.config.password.as_deref().ok_or_else(|| {
                ObsError::Authentication(
                    "OBS requires a password, but none was configured".to_string(),
                )
            })?;
            payload["authentication"] = Value::String(obs_auth_response(password, salt, challenge));
        }

        Ok(payload)
    }

    async fn request(&self, request_type: &str, request_data: Value) -> Result<Value, ObsError> {
        let timeout_ms = self.config.request_timeout_ms.max(1);
        match timeout(
            Duration::from_millis(timeout_ms),
            self.request_inner(request_type, request_data),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(ObsError::Timeout(format!(
                "{request_type} exceeded {timeout_ms}ms"
            ))),
        }
    }

    async fn request_inner(
        &self,
        request_type: &str,
        request_data: Value,
    ) -> Result<Value, ObsError> {
        let mut socket = self.connect_identified().await?;
        let request_id = Uuid::new_v4().to_string();
        send_json(
            &mut socket,
            &json!({
                "op": OP_REQUEST,
                "d": {
                    "requestType": request_type,
                    "requestId": request_id,
                    "requestData": request_data,
                }
            }),
        )
        .await?;

        loop {
            let packet = read_json(&mut socket).await?;
            match packet_op(&packet)? {
                OP_EVENT => continue,
                OP_REQUEST_RESPONSE => {
                    let Some(data) = packet.get("d") else {
                        return Err(ObsError::Protocol("response missing d payload".to_string()));
                    };
                    let response_id = data.get("requestId").and_then(Value::as_str);
                    if response_id != Some(request_id.as_str()) {
                        continue;
                    }
                    validate_request_status(data)?;
                    return Ok(data
                        .get("responseData")
                        .cloned()
                        .unwrap_or_else(|| json!({})));
                }
                op => {
                    return Err(ObsError::Protocol(format!(
                        "unexpected packet op {op} while waiting for {request_type}"
                    )));
                }
            }
        }
    }
}

#[async_trait]
impl ObsController for ObsWebSocketController {
    async fn ensure_scene_bundle(&self, bundle: &SceneBundle) -> Result<(), ObsError> {
        let response = self.request("GetSceneList", json!({})).await?;
        let existing = scene_names_from_response(&response);
        for scene in &bundle.scenes {
            if !existing.contains(&scene.name) {
                self.request("CreateScene", json!({ "sceneName": scene.name.clone() }))
                    .await?;
            }
        }
        Ok(())
    }

    async fn ensure_scene_materialization(
        &self,
        plan: &SceneMaterializationPlan,
    ) -> Result<(), ObsError> {
        self.ensure_scene_bundle(&plan.scenes).await?;
        self.ensure_source_templates(&plan.sources).await?;
        Ok(())
    }

    async fn ensure_source_templates(
        &self,
        templates: &[ObsSourceTemplate],
    ) -> Result<ObsSourceMaterializationReport, ObsError> {
        let response = self.request("GetInputList", json!({})).await?;
        let mut existing = input_names_from_response(&response);
        let mut created = Vec::new();
        let mut updated = Vec::new();

        for template in templates {
            if existing.contains(&template.input_name) {
                self.request(
                    "SetInputSettings",
                    json!({
                        "inputName": template.input_name.clone(),
                        "inputSettings": template.input_settings.clone(),
                        "overlay": true
                    }),
                )
                .await?;
                updated.push(template.input_name.clone());
            } else {
                self.request(
                    "CreateInput",
                    json!({
                        "sceneName": template.scene_name.clone(),
                        "inputName": template.input_name.clone(),
                        "inputKind": template.input_kind.clone(),
                        "inputSettings": template.input_settings.clone(),
                        "sceneItemEnabled": template.scene_item_enabled
                    }),
                )
                .await?;
                existing.insert(template.input_name.clone());
                created.push(template.input_name.clone());
            }
        }

        Ok(ObsSourceMaterializationReport {
            created,
            updated,
            skipped: Vec::new(),
            adapter: "obs-websocket".to_string(),
        })
    }

    async fn switch_scene(&self, bundle: &SceneBundle, role: SceneRole) -> Result<(), ObsError> {
        let Some(scene_name) = bundle.scene_name(role) else {
            return Err(ObsError::SceneNotFound(role.to_string()));
        };
        self.request("SetCurrentProgramScene", json!({ "sceneName": scene_name }))
            .await?;
        Ok(())
    }

    async fn start_streaming(&self) -> Result<(), ObsError> {
        self.request("StartStream", json!({})).await?;
        Ok(())
    }

    async fn stop_streaming(&self) -> Result<(), ObsError> {
        self.request("StopStream", json!({})).await?;
        Ok(())
    }

    async fn start_recording(&self) -> Result<(), ObsError> {
        self.request("StartRecord", json!({})).await?;
        Ok(())
    }

    async fn stop_recording(&self) -> Result<(), ObsError> {
        self.request("StopRecord", json!({})).await?;
        Ok(())
    }

    async fn save_replay_buffer(&self) -> Result<(), ObsError> {
        self.request("SaveReplayBuffer", json!({})).await?;
        Ok(())
    }

    async fn status(&self) -> Result<ObsStatus, ObsError> {
        let stream_status = self.request("GetStreamStatus", json!({})).await?;
        let scene_status = self.request("GetCurrentProgramScene", json!({})).await?;
        Ok(ObsStatus {
            connected: true,
            streaming: stream_status
                .get("outputActive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            current_scene: scene_status
                .get("currentProgramSceneName")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            adapter: "obs-websocket".to_string(),
        })
    }
}

fn scene_names_from_response(response: &Value) -> HashSet<String> {
    response
        .get("scenes")
        .or_else(|| response.get("sceneList"))
        .and_then(Value::as_array)
        .map(|scenes| {
            scenes
                .iter()
                .filter_map(|scene| scene.get("sceneName").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn input_names_from_response(response: &Value) -> HashSet<String> {
    response
        .get("inputs")
        .or_else(|| response.get("inputList"))
        .and_then(Value::as_array)
        .map(|inputs| {
            inputs
                .iter()
                .filter_map(|input| input.get("inputName").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn packet_op(packet: &Value) -> Result<u64, ObsError> {
    packet
        .get("op")
        .and_then(Value::as_u64)
        .ok_or_else(|| ObsError::Protocol("packet missing numeric op".to_string()))
}

async fn read_json(socket: &mut ObsSocket) -> Result<Value, ObsError> {
    while let Some(message) = socket.next().await {
        match message? {
            Message::Text(text) => return Ok(serde_json::from_str(text.as_ref())?),
            Message::Binary(bytes) => return Ok(serde_json::from_slice(bytes.as_ref())?),
            Message::Close(frame) => {
                return Err(ObsError::Connection(format!(
                    "OBS WebSocket closed: {}",
                    frame
                        .map(|value| value.reason.to_string())
                        .filter(|reason| !reason.is_empty())
                        .unwrap_or_else(|| "no reason supplied".to_string())
                )));
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => continue,
        }
    }
    Err(ObsError::Connection(
        "OBS WebSocket closed before a JSON packet was received".to_string(),
    ))
}

async fn send_json(socket: &mut ObsSocket, packet: &Value) -> Result<(), ObsError> {
    socket
        .send(Message::Text(packet.to_string().into()))
        .await
        .map_err(ObsError::from)
}

fn validate_request_status(data: &Value) -> Result<(), ObsError> {
    let Some(status) = data.get("requestStatus") else {
        return Err(ObsError::Protocol(
            "request response missing requestStatus".to_string(),
        ));
    };
    if status
        .get("result")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }
    let code = status
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let comment = status
        .get("comment")
        .and_then(Value::as_str)
        .unwrap_or("OBS request failed");
    Err(ObsError::Protocol(format!(
        "request failed: code={code} comment={comment}"
    )))
}

/// Derives the OBS WebSocket v5 authentication response.
#[must_use]
pub fn obs_auth_response(password: &str, salt: &str, challenge: &str) -> String {
    let secret_hash = Sha256::digest(format!("{password}{salt}").as_bytes());
    let secret = BASE64_STANDARD.encode(secret_hash);
    let auth_hash = Sha256::digest(format!("{secret}{challenge}").as_bytes());
    BASE64_STANDARD.encode(auth_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dry_run_switches_scene() -> Result<(), ObsError> {
        let controller = DryRunObsController::default();
        let bundle = SceneBundle::default_irl();
        controller.ensure_scene_bundle(&bundle).await?;
        controller.switch_scene(&bundle, SceneRole::Brb).await?;
        let status = controller.status().await?;
        assert_eq!(status.current_scene.as_deref(), Some("OpenIRL BRB"));
        assert_eq!(status.adapter, "dry-run");
        Ok(())
    }

    #[test]
    fn auth_response_is_deterministic() {
        let marker = std::process::id();
        let password = format!("credential-{marker}");
        let salt = format!("salt-{marker}");
        let challenge = format!("challenge-{marker}");
        let first = obs_auth_response(&password, &salt, &challenge);
        let second = obs_auth_response(&password, &salt, &challenge);
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn scene_list_parser_accepts_obs_v5_shape() {
        let response = json!({ "scenes": [{ "sceneName": "OpenIRL Live" }] });
        let names = scene_names_from_response(&response);
        assert!(names.contains("OpenIRL Live"));
    }

    #[test]
    fn input_list_parser_accepts_obs_v5_shape() {
        let response = json!({ "inputs": [{ "inputName": "OpenIRL Main Ingest" }] });
        let names = input_names_from_response(&response);
        assert!(names.contains("OpenIRL Main Ingest"));
    }

    #[test]
    fn input_list_parser_accepts_obs_v5_alternate_shape() {
        let response = json!({ "inputs": [{ "inputName": "OpenIRL Primary SRT Ingest" }] });
        let names = input_names_from_response(&response);
        assert!(names.contains("OpenIRL Primary SRT Ingest"));
    }
}
