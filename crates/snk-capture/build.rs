const COMMANDS: &[&str] = &[
    "capture_full_screen",
    "capture_window",
    "capture_region",
    "list_capturable_windows",
    "grab_screen_preview",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
