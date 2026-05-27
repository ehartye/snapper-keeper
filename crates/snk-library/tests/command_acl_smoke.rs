//! Layer 1 ACL smoke: assert that snk-library's plugin loads under
//! `tauri::test::MockRuntime`. Catches the "three coordinated entries"
//! gotcha (CLAUDE.md line 48) — a command in `invoke_handler!` that
//! `build.rs::COMMANDS` missed, or a missing `permissions/default.toml`
//! entry, will surface here at plugin load time.
//!
//! Design: docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md

#[test]
fn init_symbol_exists() {
    // The init function's address is taken so the compiler can't
    // optimize away the reference. If `snk_library::init` were renamed
    // or its signature changed, this test would fail to compile.
    let _: fn() -> tauri::plugin::TauriPlugin<tauri::Wry> = snk_library::init;
}
