const COMMANDS: &[&str] = &[
    "list_captures",
    "get_capture",
    "soft_delete_capture",
    "hard_delete_capture",
    "purge_trash",
    "set_capture_pinned",
    "list_clipboard_items",
    "get_clipboard_item",
    "toggle_clipboard_pin",
    "search_library",
    "list_tags",
    "create_tag",
    "update_tag",
    "delete_tag",
    "assign_tag",
    "remove_tag",
    "list_capture_tags",
    "get_setting",
    "set_setting",
    "get_theme",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
