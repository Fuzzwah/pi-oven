use pi_oven_protocol::Msg;

// Hand-crafted fixture shared with the TS cross-language test (task 2.6).
// Field order is intentionally non-alphabetical to verify order-tolerance.
const HELLO_FIXTURE: &str = r#"{"type":"Hello","key":"fixture-key","client_version":"0.1.0"}"#;

fn round_trip(msg: &Msg) -> Msg {
    let json = serde_json::to_string(msg).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
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
    let msg = Msg::Welcome { server_version: "1.2.3".to_string() };
    let rt = round_trip(&msg);
    assert!(matches!(rt, Msg::Welcome { ref server_version } if server_version == "1.2.3"));
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
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
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
    // A Hello with an extra unknown field must deserialise without error.
    let result: Result<Msg, _> = serde_json::from_str(
        r#"{"type":"Hello","key":"k","client_version":"0.1.0","future_field":"ignored"}"#,
    );
    assert!(result.is_ok());
}

#[test]
fn cross_language_hello_fixture() {
    // Same bytes used in packages/pi-oven-server/test/protocol.test.ts.
    let msg: Msg = serde_json::from_str(HELLO_FIXTURE).expect("parse fixture");
    assert!(matches!(msg, Msg::Hello { ref key, ref client_version }
        if key == "fixture-key" && client_version == "0.1.0"
    ));
}
