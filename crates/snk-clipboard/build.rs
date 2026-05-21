const COMMANDS: &[&str] = &["paste_item", "show_popup"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
