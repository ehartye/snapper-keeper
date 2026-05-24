//! Cross-OS sensitive-clipboard detection.
//!
//! The real implementations live in `platform/{macos,windows}.rs`. This
//! module exposes the public function the watcher calls, plus a trait
//! used by tests to inject a fake without touching the real OS clipboard.

/// Implementations of this trait answer "is the current OS clipboard
/// flagged as sensitive". Production = platform-specific OS call;
/// tests = `FakeProbe`.
pub trait SensitivityProbe: Send {
    fn is_sensitive(&self) -> bool;
}

/// Production probe — defers to the per-OS impl.
#[derive(Default)]
pub struct OsProbe;

impl SensitivityProbe for OsProbe {
    fn is_sensitive(&self) -> bool {
        crate::platform::is_sensitive()
    }
}

/// Test probe — returns a canned value. Use for unit tests of
/// downstream code (the watcher) without touching the OS clipboard.
#[derive(Debug, Clone, Copy)]
pub struct FakeProbe {
    pub answer: bool,
}

impl SensitivityProbe for FakeProbe {
    fn is_sensitive(&self) -> bool {
        self.answer
    }
}

/// Convenience for callers that don't want to manage a probe instance.
pub fn is_sensitive() -> bool {
    OsProbe.is_sensitive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_probe_returns_canned_value() {
        assert!(!FakeProbe { answer: false }.is_sensitive());
        assert!(FakeProbe { answer: true }.is_sensitive());
    }

    #[test]
    fn os_probe_delegates_to_platform_module() {
        // The Linux stub returns false; on macOS/Windows with no
        // concealed flag set, also false. Both are valid CI environments
        // for this test, so just exercise the path doesn't panic.
        let _ = OsProbe.is_sensitive();
    }
}
