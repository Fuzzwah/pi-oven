use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub workspace_id: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub seq: u64,
    pub ts: u64,
    pub event: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Msg {
    Hello {
        key: String,
        client_version: String,
    },
    Welcome {
        server_version: String,
        workspaces: Vec<WorkspaceSnapshot>,
    },
    AuthFailed {
        reason: String,
    },
    Ping {
        ts_ms: u64,
    },
    Pong {
        client_ts_ms: u64,
        server_ts_ms: u64,
    },
    Send {
        workspace_id: u64,
        text: String,
        queue_mode: String,
    },
    Abort {
        workspace_id: u64,
    },
    AgentEvent {
        workspace_id: u64,
        seq: u64,
        event: Value,
    },
    AgentStatus {
        workspace_id: u64,
        status: String,
    },
    Resume {
        workspace_id: u64,
        last_seq: u64,
    },
    ReplayBatch {
        workspace_id: u64,
        events: Vec<StoredEvent>,
        latest_seq: u64,
    },
    ErrorEvent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace_id: Option<u64>,
        reason: String,
    },
}
