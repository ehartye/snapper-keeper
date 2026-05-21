const COMMANDS: &[&str] = &[
    "list_captures",
    "get_capture",
    "soft_delete_capture",
    "list_clipboard_items",
    "get_clipboard_item",
    "toggle_clipboard_pin",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
