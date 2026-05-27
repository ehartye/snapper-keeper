//! Layer 1 plugin-init smoke for snk-annotate. See
//! snk-library/tests/command_acl_smoke.rs for design rationale.
//!
//! Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

#[test]
fn init_symbol_exists() {
    let _: fn() -> tauri::plugin::TauriPlugin<tauri::Wry> = snk_annotate::init;
}
