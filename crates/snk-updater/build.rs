const COMMANDS: &[&str] = &[
    "check_for_update",
    "get_update_status",
    "get_last_check_at",
    "restart_app",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
