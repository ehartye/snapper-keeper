//! `Redacted<T>` — wrap user-data fields at tracing call sites so they
//! print as `<redacted>` even if a log sink is later misconfigured.
//!
//! ## Why this exists
//!
//! `tracing` logs flow through user-controlled sinks (file, journald,
//! eventually external collectors). Any tracing call site that interpolates
//! clipboard content, OCR text, or file paths-as-data is one
//! misconfiguration away from leaking sensitive data to a log file the
//! user didn't expect to be readable.
//!
//! `Redacted<T>` is a cheap structural guard: even if a future call site
//! does `info!(text = %clipboard_text, "saved")`, wrapping the value as
//! `Redacted(clipboard_text)` makes the emitted record `text=<redacted>`.
//! The inner value is still accessible via `.into_inner()` / `.as_inner()`
//! for non-logging uses (storage, IPC, comparison).
//!
//! ## Usage
//!
//! ```no_run
//! use snk_library::Redacted;
//! use tracing::info;
//!
//! let user_text: String = "secret".into();
//! // Both Display and Debug print "<redacted>":
//! info!(text = %Redacted(&user_text), "clipboard event saved");
//! info!(text = ?Redacted(&user_text), "clipboard event saved");
//! ```
//!
//! The wrapper is zero-cost — it's a `#[repr(transparent)]` newtype with
//! no heap allocation, no `Clone` cost beyond the inner type, no Drop
//! beyond the inner type. The point is the `Display`/`Debug` impls.

use std::fmt;

/// Wraps a value so that `Display` and `Debug` print `"<redacted>"`
/// regardless of the inner content. Use at tracing call sites where the
/// value being logged is user data (clipboard content, OCR text, file
/// paths-as-data, secrets).
///
/// See the [module docs](self) for usage examples and rationale.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Redacted<T>(pub T);

impl<T> Redacted<T> {
    /// Construct a new `Redacted` wrapper around `value`.
    pub fn new(value: T) -> Self {
        Redacted(value)
    }

    /// Consume the wrapper and return the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Borrow the inner value.
    pub fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(value: T) -> Self {
        Redacted(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_prints_redacted_for_string() {
        let r = Redacted::new(String::from("super-secret-password"));
        assert_eq!(format!("{r}"), "<redacted>");
    }

    #[test]
    fn debug_prints_redacted_for_string() {
        let r = Redacted::new(String::from("super-secret-password"));
        assert_eq!(format!("{r:?}"), "<redacted>");
    }

    #[test]
    fn display_prints_redacted_for_numbers() {
        let r = Redacted::new(424242u64);
        assert_eq!(format!("{r}"), "<redacted>");
    }

    #[test]
    fn debug_prints_redacted_for_complex_types() {
        // Inner type's Debug impl is bypassed — wrapper's impl wins.
        let r = Redacted::new(vec![1, 2, 3, 4]);
        assert_eq!(format!("{r:?}"), "<redacted>");
    }

    #[test]
    fn into_inner_returns_value() {
        let r = Redacted::new(String::from("inner"));
        assert_eq!(r.into_inner(), "inner");
    }

    #[test]
    fn as_inner_borrows_value() {
        let r = Redacted::new(String::from("inner"));
        assert_eq!(r.as_inner(), "inner");
        // wrapper still usable after as_inner borrow
        assert_eq!(format!("{r}"), "<redacted>");
    }

    #[test]
    fn from_impl_works_for_implicit_conversion() {
        let r: Redacted<&'static str> = "secret".into();
        assert_eq!(format!("{r}"), "<redacted>");
    }

    #[test]
    fn equality_compares_inner_values() {
        let a = Redacted::new("same");
        let b = Redacted::new("same");
        let c = Redacted::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn wrapper_is_repr_transparent_for_same_size_as_inner() {
        // Sanity check that the wrapper is zero-cost in memory.
        use std::mem::size_of;
        assert_eq!(size_of::<Redacted<u32>>(), size_of::<u32>());
        assert_eq!(size_of::<Redacted<String>>(), size_of::<String>());
        assert_eq!(size_of::<Redacted<Vec<u8>>>(), size_of::<Vec<u8>>());
    }

    #[test]
    fn redacted_does_not_leak_in_format_args_chain() {
        // Common tracing pattern: build a record via fmt::Arguments. Wrap
        // the value once; downstream interpolation must also redact.
        let value = String::from("password=hunter2");
        let r = Redacted::new(&value);
        let formatted = format!("event x detail={r}");
        assert!(!formatted.contains("hunter2"));
        assert!(!formatted.contains("password"));
        assert!(formatted.contains("<redacted>"));
    }
}
