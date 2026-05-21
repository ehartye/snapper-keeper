const COMMANDS: &[&str] = &["ocr_status"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
