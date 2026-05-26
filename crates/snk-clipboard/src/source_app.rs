//! Per-OS detection of "which app wrote to the clipboard".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAppKind {
    MacosBundleId,
    WindowsExe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceApp {
    pub identifier: String,
    pub display_name: String,
    pub kind: SourceAppKind,
}

impl SourceApp {
    /// Whether two identifiers refer to the same app — case rules differ
    /// per `kind` to match OS norms.
    pub fn identifier_matches(&self, other: &str) -> bool {
        match self.kind {
            SourceAppKind::WindowsExe => self.identifier.eq_ignore_ascii_case(other),
            SourceAppKind::MacosBundleId => self.identifier == other,
        }
    }
}

pub fn current() -> Option<SourceApp> {
    crate::platform::current_source_app()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_identifier_match_is_case_insensitive() {
        let app = SourceApp {
            identifier: "1Password.exe".into(),
            display_name: "1Password".into(),
            kind: SourceAppKind::WindowsExe,
        };
        assert!(app.identifier_matches("1password.exe"));
        assert!(app.identifier_matches("1PASSWORD.EXE"));
        assert!(!app.identifier_matches("KeePass.exe"));
    }

    #[test]
    fn macos_identifier_match_is_case_sensitive() {
        let app = SourceApp {
            identifier: "com.1password.1password8".into(),
            display_name: "1Password 8".into(),
            kind: SourceAppKind::MacosBundleId,
        };
        assert!(app.identifier_matches("com.1password.1password8"));
        assert!(!app.identifier_matches("COM.1password.1password8"));
        assert!(!app.identifier_matches("com.bitwarden.desktop"));
    }
}
