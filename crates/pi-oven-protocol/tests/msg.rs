use pi_oven_protocol::Msg;
use std::path::PathBuf;

// Hand-crafted fixture shared with the TS cross-language test (task 2.6).
// Field order is intentionally non-alphabetical to verify order-tolerance.
const HELLO_FIXTURE: &str = r#"{"type":"Hello","key":"fixture-key","client_version":"0.1.0"}"#;

fn round_trip(msg: &Msg) -> Msg {
    let json = serde_json::to_string(msg).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

fn fixtures_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest)
        .join("../../packages/pi-oven-server/test/fixtures/protocol")
}

fn round_trip_fixture(name: &str) {
    let path = fixtures_dir().join(format!("{name}.json"));
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let msg: Msg = serde_json::from_str(json.trim())
        .unwrap_or_else(|e| panic!("parse {name}.json: {e}"));
    let reencoded = serde_json::to_string(&msg)
        .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
    let v1: serde_json::Value = serde_json::from_str(&reencoded).unwrap();
    let rt: Msg = serde_json::from_str(&reencoded).unwrap();
    let v2: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&rt).unwrap()).unwrap();
    assert_eq!(v1, v2, "{name} round-trip value mismatch");
}

#[test]
fn hello_round_trip() {
    let msg = Msg::Hello {
        key: "abc".to_string(),
        client_version: "0.1.0".to_string(),
    };
    let rt = round_trip(&msg);
    assert!(matches!(rt, Msg::Hello { ref key, .. } if key == "abc"));
}

#[test]
fn welcome_round_trip() {
    let msg = Msg::Welcome {
        server_version: "1.2.3".to_string(),
        workspaces: vec![],
    };
    let rt = round_trip(&msg);
    assert!(
        matches!(rt, Msg::Welcome { ref server_version, .. } if server_version == "1.2.3")
    );
}

#[test]
fn auth_failed_round_trip() {
    let msg = Msg::AuthFailed { reason: "invalid_key".to_string() };
    let rt = round_trip(&msg);
    assert!(matches!(rt, Msg::AuthFailed { ref reason } if reason == "invalid_key"));
}

#[test]
fn ping_round_trip() {
    let msg = Msg::Ping { ts_ms: 12345 };
    let rt = round_trip(&msg);
    assert!(matches!(rt, Msg::Ping { ts_ms: 12345 }));
}

#[test]
fn pong_round_trip() {
    let msg = Msg::Pong { client_ts_ms: 100, server_ts_ms: 200 };
    let rt = round_trip(&msg);
    assert!(matches!(rt, Msg::Pong { client_ts_ms: 100, server_ts_ms: 200 }));
}

#[test]
fn type_field_is_the_tag() {
    let msg = Msg::Hello {
        key: "k".to_string(),
        client_version: "0.0.0".to_string(),
    };
    let json: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
    assert_eq!(json["type"], "Hello");
    assert_eq!(json["key"], "k");
}

#[test]
fn unknown_type_is_rejected() {
    let result: Result<Msg, _> = serde_json::from_str(r#"{"type":"DefinitelyNotAMessage"}"#);
    assert!(result.is_err());
}

#[test]
fn forward_compat_extra_fields_accepted() {
    let result: Result<Msg, _> = serde_json::from_str(
        r#"{"type":"Hello","key":"k","client_version":"0.1.0","future_field":"ignored"}"#,
    );
    assert!(result.is_ok());
}

#[test]
fn cross_language_hello_fixture() {
    let msg: Msg = serde_json::from_str(HELLO_FIXTURE).expect("parse fixture");
    assert!(matches!(msg, Msg::Hello { ref key, ref client_version }
        if key == "fixture-key" && client_version == "0.1.0"
    ));
}

// Task 1.6: fixture round-trip tests for new message variants

#[test]
fn fixture_send() {
    round_trip_fixture("Send");
}

#[test]
fn fixture_abort() {
    round_trip_fixture("Abort");
}

#[test]
fn fixture_agent_event() {
    round_trip_fixture("AgentEvent");
}

#[test]
fn fixture_agent_status() {
    round_trip_fixture("AgentStatus");
}

#[test]
fn fixture_resume() {
    round_trip_fixture("Resume");
}

#[test]
fn fixture_replay_batch() {
    round_trip_fixture("ReplayBatch");
}

#[test]
fn fixture_error_event() {
    round_trip_fixture("ErrorEvent");
}

#[test]
fn fixture_welcome() {
    round_trip_fixture("Welcome");
}
