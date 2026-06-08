const COMMANDS: &[&str] = &[
    "paste_item",
    "show_popup",
    "detect_frontmost_app",
    "clipboard_status",
    "clipboard_permission_status",
    "open_accessibility_settings",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
