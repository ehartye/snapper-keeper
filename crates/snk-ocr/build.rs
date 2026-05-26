const COMMANDS: &[&str] = &["ocr_status", "get_ocr_words"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
