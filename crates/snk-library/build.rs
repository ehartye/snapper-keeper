const COMMANDS: &[&str] = &["list_captures", "get_capture"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
