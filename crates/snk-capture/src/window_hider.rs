//! Hide/restore the app's own windows around a screen grab so the
//! captured pixels don't include snapper-keeper UI. Window enumeration
//! is abstracted behind `WindowManager` so the snapshot/restore logic
//! can be unit-tested without a real Tauri runtime.

use tauri::{AppHandle, Manager, Runtime};
use tracing::warn;

/// Abstracts the per-window operations the guard needs, so tests can
/// drive snapshot/restore logic without a real Tauri runtime.
pub trait WindowManager {
    /// Returns (label, currently-visible) for every webview window
    /// the app owns.
    fn list_own_windows(&self) -> Vec<(String, bool)>;
    fn hide(&self, label: &str);
    fn show(&self, label: &str);
}

/// Tauri-backed implementation. Used in production.
pub struct TauriWindowManager<'a, R: Runtime> {
    app: &'a AppHandle<R>,
}

impl<'a, R: Runtime> TauriWindowManager<'a, R> {
    pub fn new(app: &'a AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<'a, R: Runtime> WindowManager for TauriWindowManager<'a, R> {
    fn list_own_windows(&self) -> Vec<(String, bool)> {
        self.app
            .webview_windows()
            .into_iter()
            .map(|(label, win)| {
                let visible = win.is_visible().unwrap_or(false);
                (label, visible)
            })
            .collect()
    }

    fn hide(&self, label: &str) {
        if let Some(win) = self.app.get_webview_window(label) {
            if let Err(e) = win.hide() {
                warn!(label, error = %e, "failed to hide window for capture");
            }
        }
    }

    fn show(&self, label: &str) {
        if let Some(win) = self.app.get_webview_window(label) {
            if let Err(e) = win.show() {
                warn!(label, error = %e, "failed to restore window after capture");
            }
        }
    }
}

/// RAII guard that hides a set of windows on construction and restores
/// them on `Drop`. Restoring in Drop guarantees the windows come back
/// even if the grab panics or returns an error.
pub struct WindowVisibilityGuard<'a, W: WindowManager> {
    manager: &'a W,
    hidden_labels: Vec<String>,
}

impl<'a, W: WindowManager> WindowVisibilityGuard<'a, W> {
    /// Snapshot current visibility, hide every visible window NOT in
    /// `exclude_labels`, return a guard that will restore them on drop.
    pub fn hide_all(manager: &'a W, exclude_labels: &[&str]) -> Self {
        let mut hidden = Vec::new();
        for (label, visible) in manager.list_own_windows() {
            if !visible {
                continue;
            }
            if exclude_labels.iter().any(|ex| *ex == label) {
                continue;
            }
            manager.hide(&label);
            hidden.push(label);
        }
        Self {
            manager,
            hidden_labels: hidden,
        }
    }
}

impl<'a, W: WindowManager> Drop for WindowVisibilityGuard<'a, W> {
    fn drop(&mut self) {
        for label in &self.hidden_labels {
            self.manager.show(label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Records every hide/show call so tests can assert on ordering +
    /// final state without driving a real Tauri runtime.
    struct MockWindowManager {
        visibility: RefCell<HashMap<String, bool>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockWindowManager {
        fn new(initial: &[(&str, bool)]) -> Self {
            let map = initial.iter().map(|(k, v)| (k.to_string(), *v)).collect();
            Self {
                visibility: RefCell::new(map),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl WindowManager for MockWindowManager {
        fn list_own_windows(&self) -> Vec<(String, bool)> {
            self.visibility
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect()
        }

        fn hide(&self, label: &str) {
            self.calls.borrow_mut().push(format!("hide:{label}"));
            self.visibility
                .borrow_mut()
                .insert(label.to_string(), false);
        }

        fn show(&self, label: &str) {
            self.calls.borrow_mut().push(format!("show:{label}"));
            self.visibility.borrow_mut().insert(label.to_string(), true);
        }
    }

    #[test]
    fn guard_hides_visible_windows_on_construction() {
        let mgr = MockWindowManager::new(&[("library", true), ("settings", true)]);
        let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&false));
        assert_eq!(viz.get("settings"), Some(&false));
    }

    #[test]
    fn guard_does_not_touch_already_hidden_windows() {
        let mgr = MockWindowManager::new(&[("library", true), ("popup", false)]);
        let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        let calls = mgr.calls();
        assert!(calls.contains(&"hide:library".to_string()));
        assert!(!calls.contains(&"hide:popup".to_string()));
    }

    #[test]
    fn guard_restores_visibility_on_drop() {
        let mgr = MockWindowManager::new(&[("library", true), ("settings", true)]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
            let viz = mgr.visibility.borrow();
            assert_eq!(viz.get("library"), Some(&false));
        }
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&true));
        assert_eq!(viz.get("settings"), Some(&true));
    }

    #[test]
    fn guard_only_restores_what_it_hid() {
        let mgr = MockWindowManager::new(&[("library", true), ("popup", false)]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        }
        let calls = mgr.calls();
        assert!(calls.contains(&"show:library".to_string()));
        assert!(!calls.contains(&"show:popup".to_string()));
    }

    #[test]
    fn guard_respects_exclude_labels() {
        let mgr = MockWindowManager::new(&[("library", true), ("capture-overlay", true)]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &["capture-overlay"]);
            let viz = mgr.visibility.borrow();
            assert_eq!(viz.get("capture-overlay"), Some(&true));
            assert_eq!(viz.get("library"), Some(&false));
        }
        let calls = mgr.calls();
        assert!(!calls.iter().any(|c| c.contains("capture-overlay")));
    }

    #[test]
    fn guard_restores_even_after_panic_in_scope() {
        let mgr = MockWindowManager::new(&[("library", true)]);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
            panic!("simulated grab failure");
        }));
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&true));
    }
}
