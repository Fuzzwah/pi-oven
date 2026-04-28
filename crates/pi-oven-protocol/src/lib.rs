//! pi-oven wire protocol types. Stub for the scaffold-runtime change; real
//! `Msg` variants land alongside the WebSocket transport change.

use serde::{Deserialize, Serialize};

/// Placeholder wire message. Real variants will be added by future changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Msg {
    /// No-op placeholder so the type compiles before any real variants exist.
    #[serde(rename = "noop")]
    Noop,
}
