use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use pi_oven_protocol::Msg;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const PING_INTERVAL: Duration = Duration::from_secs(20);
const MAX_MISSED_PONGS: u32 = 2;

pub struct CloseInfo {
    pub code: u16,
    pub reason: String,
}

pub struct ClientHandle {
    /// Send outgoing messages to the server.
    pub tx: mpsc::Sender<Msg>,
    /// Receive incoming messages from the server.
    pub rx: mpsc::Receiver<Msg>,
    /// Resolves when the connection drops; carries the close code.
    pub close: tokio::sync::oneshot::Receiver<CloseInfo>,
}

pub struct Client;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

impl Client {
    /// Open the WebSocket, send Hello, await Welcome.
    /// Returns Err if auth fails or the connection drops before Welcome.
    pub async fn connect(
        url: &str,
        shared_key: &str,
        client_version: &str,
    ) -> Result<ClientHandle> {
        let (ws_stream, _) = connect_async(url)
            .await
            .context("WebSocket connect failed")?;
        let (mut write, mut read) = ws_stream.split();

        let hello = serde_json::to_string(&Msg::Hello {
            key: shared_key.to_string(),
            client_version: client_version.to_string(),
        })?;
        write
            .send(Message::Text(hello.into()))
            .await
            .context("send Hello")?;

        // Wait for Welcome (or AuthFailed / close).
        let welcome: Msg = loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    let msg: Msg = serde_json::from_str(&text)
                        .context("parse server message during handshake")?;
                    match msg {
                        Msg::Welcome { .. } => break msg,
                        Msg::AuthFailed { reason } => {
                            return Err(anyhow!("auth failed: {reason}"));
                        }
                        _ => {
                            return Err(anyhow!("unexpected message during handshake: {:?}", msg));
                        }
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    let code = frame
                        .as_ref()
                        .map(|f| u16::from(f.code))
                        .unwrap_or(0);
                    return Err(anyhow!("connection closed during handshake: code {code}"));
                }
                Some(Ok(_)) => continue, // ignore ping/pong/binary control frames
                Some(Err(e)) => return Err(e.into()),
                None => return Err(anyhow!("connection closed before Welcome")),
            }
        };

        // Handshake succeeded — wire up channels.
        let (out_tx, mut out_rx) = mpsc::channel::<Msg>(64);
        let (in_tx, in_rx) = mpsc::channel::<Msg>(64);
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<CloseInfo>();

        // Forward Welcome to the UI layer so it can send Resume.
        let _ = in_tx.send(welcome).await;

        tokio::spawn(async move {
            let mut ping_interval = tokio::time::interval(PING_INTERVAL);
            ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut missed_pongs: u32 = 0;
            let mut pending_ping: bool = false;

            let close_info = loop {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<Msg>(&text) {
                                    Ok(Msg::Pong { .. }) => {
                                        missed_pongs = 0;
                                        pending_ping = false;
                                    }
                                    Ok(m) => {
                                        let _ = in_tx.send(m).await;
                                    }
                                    Err(_) => {}
                                }
                            }
                            Some(Ok(Message::Close(frame))) => {
                                let code = frame.as_ref().map(|f| u16::from(f.code)).unwrap_or(0);
                                let reason = frame
                                    .and_then(|f| Some(f.reason.to_string()))
                                    .unwrap_or_default();
                                break CloseInfo { code, reason };
                            }
                            Some(Ok(_)) => {}
                            Some(Err(e)) => {
                                tracing::debug!(?e, "WebSocket read error");
                                break CloseInfo { code: 0, reason: e.to_string() };
                            }
                            None => {
                                break CloseInfo { code: 0, reason: "connection closed".into() };
                            }
                        }
                    }
                    msg = out_rx.recv() => {
                        match msg {
                            Some(m) => {
                                if let Ok(json) = serde_json::to_string(&m) {
                                    let _ = write.send(Message::Text(json.into())).await;
                                }
                            }
                            None => break CloseInfo { code: 1000, reason: "channel closed".into() },
                        }
                    }
                    _ = ping_interval.tick() => {
                        if pending_ping {
                            missed_pongs += 1;
                            if missed_pongs >= MAX_MISSED_PONGS {
                                tracing::warn!("heartbeat: {missed_pongs} consecutive missed pongs — dropping connection");
                                break CloseInfo { code: 0, reason: "missed pongs".into() };
                            }
                        }
                        let ts = now_ms();
                        let ping = Msg::Ping { ts_ms: ts };
                        if let Ok(json) = serde_json::to_string(&ping) {
                            let _ = write.send(Message::Text(json.into())).await;
                        }
                        pending_ping = true;
                    }
                }
            };

            let _ = close_tx.send(close_info);
        });

        Ok(ClientHandle { tx: out_tx, rx: in_rx, close: close_rx })
    }
}
