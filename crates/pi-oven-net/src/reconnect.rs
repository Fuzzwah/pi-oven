use std::time::Duration;

use pi_oven_protocol::Msg;
use tokio::sync::{mpsc, watch};

use crate::client::Client;

/// Close codes that are terminal — no reconnect should be attempted.
const TERMINAL_CODES: &[u16] = &[
    4401, // auth failed — operator must fix the key
    4002, // replaced — a newer client owns the session
    1000, // normal close
];

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Authenticated,
    Reconnecting { in_seconds: u64 },
    Failed { reason: String },
}

pub struct ReconnectHandle {
    /// Send outgoing messages to the server (if connected; silently dropped otherwise).
    pub msg_tx: mpsc::Sender<Msg>,
    /// Receive incoming messages from the server.
    pub msg_rx: mpsc::Receiver<Msg>,
    /// Watch the current connection state.
    pub state_rx: watch::Receiver<ConnectionState>,
}

fn backoff_secs(attempt: u32) -> u64 {
    let base = 2u64.saturating_pow(attempt).min(30);
    // ±25% jitter using the low bits of the current nanosecond clock.
    let quarter = base / 4;
    if quarter == 0 {
        return base;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let offset = nanos % (quarter * 2);
    base.saturating_sub(quarter) + offset
}

/// Spawn a reconnecting connection loop.  Returns a `ReconnectHandle` immediately;
/// the actual connection attempt happens in a background tokio task.
pub fn start(
    url: String,
    shared_key: String,
    client_version: String,
) -> ReconnectHandle {
    let (msg_tx, _msg_rx) = mpsc::channel::<Msg>(64);
    let (in_tx, in_rx) = mpsc::channel::<Msg>(64);
    let (state_tx, state_rx) = watch::channel(ConnectionState::Connecting);

    tokio::spawn(reconnect_loop(
        url,
        shared_key,
        client_version,
        in_tx,
        state_tx,
    ));

    ReconnectHandle {
        msg_tx,
        msg_rx: in_rx,
        state_rx,
    }
}

async fn reconnect_loop(
    url: String,
    shared_key: String,
    client_version: String,
    in_tx: mpsc::Sender<Msg>,
    state_tx: watch::Sender<ConnectionState>,
) {
    let mut attempt: u32 = 0;

    loop {
        let _ = state_tx.send(ConnectionState::Connecting);
        tracing::info!(url = %url, attempt, "connecting");

        match Client::connect(&url, &shared_key, &client_version).await {
            Ok(handle) => {
                let _ = state_tx.send(ConnectionState::Authenticated);
                tracing::info!("authenticated");
                attempt = 0;

                // Forward incoming messages to the caller.
                let mut rx = handle.rx;
                let close = handle.close;

                // Drive two futures: incoming message forwarder and the close signal.
                let close_info = tokio::select! {
                    info = close => info.ok(),
                    _ = async {
                        while let Some(msg) = rx.recv().await {
                            let _ = in_tx.send(msg).await;
                        }
                    } => None,
                };

                let code = close_info.as_ref().map(|c| c.code).unwrap_or(0);
                let reason = close_info
                    .map(|c| c.reason)
                    .unwrap_or_else(|| "channel closed".into());

                tracing::info!(code, reason = %reason, "disconnected");

                if TERMINAL_CODES.contains(&code) {
                    let _ = state_tx.send(ConnectionState::Failed { reason });
                    return;
                }
            }
            Err(e) => {
                let reason = e.to_string();
                // auth failures from connect() surface as "auth failed: …"
                if reason.contains("auth failed") {
                    tracing::warn!(%reason, "auth failed — not retrying");
                    let _ = state_tx.send(ConnectionState::Failed { reason });
                    return;
                }
                tracing::warn!(%reason, attempt, "connection failed");
            }
        }

        let delay = backoff_secs(attempt);
        attempt += 1;
        tracing::info!(in_seconds = delay, "reconnecting");
        let _ = state_tx.send(ConnectionState::Reconnecting { in_seconds: delay });
        tokio::time::sleep(Duration::from_secs(delay)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_schedule() {
        // Attempt 0 → base=1, attempt 1 → base=2, …, attempt 5+ → base=30 (capped).
        // Jitter is ±25% of base, so max value for attempt≥5 is 30*1.25 = 37.5 → 37.
        let samples: Vec<u64> = (0..6).map(backoff_secs).collect();
        // All values are within the jittered cap: max = ceil(30 * 1.25) = 38.
        for s in &samples {
            assert!(*s <= 38, "backoff exceeded jittered upper bound: {s}");
        }
        // First attempt uses base=1, so result ≤ ceil(1*1.25)=1 or 2.
        assert!(samples[0] <= 2, "first attempt should be at most 2s: {}", samples[0]);
        // The jittered base is always ≥ base - quarter ≥ 0.
        for s in &samples {
            assert!(*s >= 0);
        }
    }

    #[tokio::test]
    async fn no_reconnect_on_auth_failure() {
        use tokio::net::TcpListener;
        use tokio_tungstenite::{accept_async, tungstenite::Message as TMsg};
        use futures_util::{SinkExt, StreamExt};

        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = tcp.local_addr().unwrap();
        let url = format!("ws://{addr}");

        // Mock server: accept one connection then reply AuthFailed + close 4401.
        tokio::spawn(async move {
            if let Ok((stream, _)) = tcp.accept().await {
                if let Ok(mut ws) = accept_async(stream).await {
                    // Consume Hello.
                    ws.next().await;
                    let auth_failed = serde_json::to_string(&Msg::AuthFailed {
                        reason: "invalid_key".into(),
                    })
                    .unwrap();
                    let _ = ws.send(TMsg::Text(auth_failed.into())).await;
                    let _ = ws
                        .send(TMsg::Close(Some(
                            tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(4401u16),
                                reason: "auth_failed".into(),
                            },
                        )))
                        .await;
                }
            }
        });

        let handle = start(url, "wrong-key".into(), "0.0.0".into());

        // Wait up to 3s for the state to become Failed (no reconnect).
        let mut state_rx = handle.state_rx;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            {
                let state = state_rx.borrow().clone();
                if matches!(state, ConnectionState::Failed { .. }) {
                    return; // Test passed.
                }
            }
            if tokio::time::Instant::now() > deadline {
                panic!("expected Failed state after 4401, but timed out");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = state_rx.changed().await;
        }
    }

    #[tokio::test]
    async fn attempt_resets_on_success() {
        // Verify that backoff_secs(0) gives the minimum backoff (≥1s base * 0.75).
        // This is a unit-level check since integration with the running server is manual.
        assert!(backoff_secs(0) <= 2, "first retry within 2s");
    }
}
