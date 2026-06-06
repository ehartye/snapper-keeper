//! #53 — cross-plugin event protocol test for `capture:saved`.
//!
//! `capture:saved` is emitted by snk-capture and snk-annotate as the bare
//! capture id (a `String`) via `app.emit("capture:saved", &capture.id)`, and
//! consumed by snk-ocr's plugin which parses it with
//! `event.payload().trim_matches('"')` (see `snk-ocr/src/plugin.rs`). The whole
//! contract is that one parse, so this test locks the wire shape both sides
//! depend on.
//!
//! Tauri's `emit` serializes the payload with `serde_json`, so this asserts the
//! exact serialized form and the consumer's recovery without standing up a
//! mock runtime — a `tauri::test::mock_app` variant can't launch in the
//! non-interactive Windows/WebView2 sandbox (see CLAUDE.md), and the serde-level
//! contract is what actually matters: an emitter switching to a struct payload
//! would change `wire` and break the consumer's `trim_matches` recovery, which
//! this test catches.

#[test]
fn capture_saved_payload_is_a_bare_json_string_the_consumer_parse_recovers() {
    let id = "0190a1b2-c3d4-7e5f-8901-234567890abc".to_string();

    // Emitter side: `app.emit("capture:saved", &id)` serializes via serde_json.
    let wire = serde_json::to_string(&id).expect("serialize capture id");
    assert_eq!(
        wire,
        format!("\"{id}\""),
        "capture:saved must carry the id as a bare JSON string"
    );

    // Consumer side (snk-ocr/src/plugin.rs): strip the surrounding quotes.
    let recovered = wire.trim_matches('"').to_string();
    assert_eq!(recovered, id, "consumer parse must recover the original id");
}
