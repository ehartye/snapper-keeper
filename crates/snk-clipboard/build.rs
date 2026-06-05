const COMMANDS: &[&str] = &[
    "paste_item",
    "show_popup",
    "detect_frontmost_app",
    "clipboard_status",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
