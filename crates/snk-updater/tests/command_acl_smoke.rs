//! Layer 1 plugin-init smoke for snk-updater. See
//! snk-library/tests/command_acl_smoke.rs for design rationale.
//!
//! Note: lib name is `snk_releaser` not `snk_updater` — Windows UAC
//! installer-detection heuristic flags any binary filename containing
//! "update", "setup", or "install" (see Cargo.toml header comment).
//!
//! Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

#[test]
fn init_symbol_exists() {
    let _: fn() -> tauri::plugin::TauriPlugin<tauri::Wry> = snk_releaser::init;
}
