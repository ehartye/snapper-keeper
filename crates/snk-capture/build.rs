const COMMANDS: &[&str] = &["capture_full_screen"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
