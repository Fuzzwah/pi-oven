use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Msg {
    Hello { key: String, client_version: String },
    Welcome { server_version: String },
    AuthFailed { reason: String },
    Ping { ts_ms: u64 },
    Pong { client_ts_ms: u64, server_ts_ms: u64 },
}
