//! User-managed app-blocklist filter for the clipboard watcher.
//!
//! Reads the `clipboard.app_blocklist` setting via snk-library. Setting
//! shape is a JSON array of `BlocklistEntry`. Match is delegated to
//! `SourceApp::identifier_matches` so OS-specific case rules apply.

use serde::{Deserialize, Serialize};
use tracing::warn;

use snk_library::{settings, Db};

use crate::source_app::{SourceApp, SourceAppKind};

const SETTING_KEY: &str = "clipboard.app_blocklist";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub identifier: String,
    pub display_name: String,
    pub kind: SourceAppKind,
}

/// Returns true if `source` matches an entry in the persisted blocklist.
///
/// Fail-open: an unset setting, an empty array, or a malformed JSON value
/// all return false. The watcher therefore degrades to "OS flag only"
/// rather than failing the entire event loop.
pub fn matches(db: &Db, source: &SourceApp) -> bool {
    let raw = match settings::get(db, SETTING_KEY) {
        Ok(Some(v)) => v,
        Ok(None) => return false,
        Err(e) => {
            warn!(error = ?e, "blocklist setting read failed; treating as empty");
            return false;
        }
    };
    let entries: Vec<BlocklistEntry> = match serde_json::from_value(raw) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "blocklist setting malformed; treating as empty");
            return false;
        }
    };
    entries
        .iter()
        .any(|e| e.kind == source.kind && source.identifier_matches(&e.identifier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use snk_library::settings;

    // Reuse snk-library's test_support::fresh_db pattern via a tiny local
    // helper — that helper is private to snk-library, so we mint our own
    // here against the same crate's public API.
    fn fresh_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        let db = Db::open(&path).unwrap();
        (dir, db)
    }

    fn mac(id: &str) -> SourceApp {
        SourceApp {
            identifier: id.into(),
            display_name: id.into(),
            kind: SourceAppKind::MacosBundleId,
        }
    }

    fn win(id: &str) -> SourceApp {
        SourceApp {
            identifier: id.into(),
            display_name: id.into(),
            kind: SourceAppKind::WindowsExe,
        }
    }

    #[test]
    fn returns_false_when_setting_unset() {
        let (_t, db) = fresh_db();
        assert!(!matches(&db, &mac("com.x.y")));
    }

    #[test]
    fn returns_false_when_setting_is_empty_array() {
        let (_t, db) = fresh_db();
        settings::set(&db, SETTING_KEY, &json!([])).unwrap();
        assert!(!matches(&db, &win("foo.exe")));
    }

    #[test]
    fn returns_true_on_exact_match_macos() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "com.1password.1password8",
                "display_name": "1Password 8",
                "kind": "macos_bundle_id"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &mac("com.1password.1password8")));
        assert!(!matches(&db, &mac("com.bitwarden.desktop")));
    }

    #[test]
    fn windows_match_is_case_insensitive() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "1Password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &win("1password.exe")));
        assert!(matches(&db, &win("1PASSWORD.EXE")));
    }

    #[test]
    fn macos_match_is_case_sensitive() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "com.example.app",
                "display_name": "App",
                "kind": "macos_bundle_id"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &mac("com.example.app")));
        assert!(!matches(&db, &mac("Com.Example.App")));
    }

    #[test]
    fn cross_kind_entries_are_inert() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "1Password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        // macOS source can't match a windows_exe entry.
        assert!(!matches(&db, &mac("1Password.exe")));
    }

    #[test]
    fn malformed_json_falls_open() {
        let (_t, db) = fresh_db();
        // Plant a non-array JSON value directly.
        settings::set(&db, SETTING_KEY, &json!({"not": "an array"})).unwrap();
        assert!(!matches(&db, &win("foo.exe")));
    }
}
