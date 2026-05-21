const COMMANDS: &[&str] = &["check_for_update", "get_update_status"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
