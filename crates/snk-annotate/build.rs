const COMMANDS: &[&str] = &["save_annotation", "derive_capture"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
