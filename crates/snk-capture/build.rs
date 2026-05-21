const COMMANDS: &[&str] = &[
    "capture_full_screen",
    "capture_window",
    "capture_region",
    "list_capturable_windows",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
