const COMMANDS: &[&str] = &["save_annotation"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
