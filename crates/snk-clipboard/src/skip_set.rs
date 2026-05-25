//! Skip set: content-hash-based "don't re-record my own clipboard
//! writes" mechanism. Replaces the previous AtomicBool that could
//! only skip ONE next event and had no notion of which event to
//! skip.
//!
//! Per [#38](https://github.com/ehartye/snapper-keeper/issues/38):
//! when the app writes to the clipboard (e.g. \`paste_item\` reads a
//! history item back into the clipboard), the watcher would otherwise
//! immediately re-record what we just wrote. The previous \`SKIP_NEXT\`
//! AtomicBool worked for the common case but had two failure modes:
//! 1. Race: watcher already processed the next event before SKIP_NEXT
//!    was set.
//! 2. Untestable: no hash association meant tests couldn't distinguish
//!    "skipped our own write" from any other skip.
//!
//! This module:
//! - Hashes the content (SHA-256 → u64) before clipboard write
//! - Remembers (hash, timestamp) pairs in a per-process set
//! - Watcher checks the set on every event; matches within TTL skip
//! - TTL-only pruning (no explicit size cap) — at 5s TTL and typical
//!   paste cadence, set stays bounded naturally

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// How long a content hash remains in the skip set after being
/// emitted. Generous enough for WM_CLIPBOARDUPDATE handler latency
/// (<100ms typical) plus pathological lag; short enough that legitimate
/// user re-copies of the same content don't get silently swallowed.
pub const SKIP_TTL: Duration = Duration::from_secs(5);

static SKIPPED: OnceLock<Mutex<Vec<(u64, Instant)>>> = OnceLock::new();

fn set() -> &'static Mutex<Vec<(u64, Instant)>> {
    SKIPPED.get_or_init(|| Mutex::new(Vec::new()))
}

/// Hash arbitrary bytes to a u64. Uses SHA-256 truncated to 8 bytes.
/// `sha2` is already a workspace dep; consistent with the existing
/// library hashing patterns. Collision probability for ~50 concurrent
/// entries is negligible (2^-30 range).
pub fn hash_content(bytes: &[u8]) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

/// Mark a content hash as "emitted by us" so the watcher will skip
/// the next observed event with that hash (within `SKIP_TTL`).
pub fn mark_emitted(hash: u64) {
    let mut s = set().lock().expect("skip set mutex poisoned");
    prune_expired(&mut s);
    s.push((hash, Instant::now()));
}

/// Returns true if the given hash was recently marked emitted by us
/// and the TTL hasn't expired. Removes the matched entry on hit
/// (one-shot skip — second occurrence of the same content within TTL
/// is treated as a real new event, since the user may have copied the
/// same thing again on purpose).
pub fn should_skip(hash: u64) -> bool {
    let mut s = set().lock().expect("skip set mutex poisoned");
    prune_expired(&mut s);
    if let Some(idx) = s.iter().position(|(h, _)| *h == hash) {
        s.swap_remove(idx);
        true
    } else {
        false
    }
}

fn prune_expired(s: &mut Vec<(u64, Instant)>) {
    let now = Instant::now();
    s.retain(|(_, ts)| now.duration_since(*ts) < SKIP_TTL);
}

/// Test-only: clear all skip entries. Used by tests that need a
/// guaranteed-empty starting state since the set is process-global.
#[cfg(test)]
pub(crate) fn clear_for_test() {
    let mut s = set().lock().expect("skip set mutex poisoned");
    s.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::thread;

    fn unique_hash() -> u64 {
        // Use system time nanos to get a different hash per test to
        // avoid cross-test interference even with `clear_for_test`.
        hash_content(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_be_bytes()
                .as_slice(),
        )
    }

    #[test]
    #[serial(skip_set)]
    fn mark_then_check_returns_skip_true() {
        clear_for_test();
        let h = unique_hash();
        mark_emitted(h);
        assert!(should_skip(h), "hash just marked should be a skip");
    }

    #[test]
    #[serial(skip_set)]
    fn unknown_hash_does_not_skip() {
        clear_for_test();
        let h = unique_hash();
        // Don't mark; check directly.
        assert!(!should_skip(h));
    }

    #[test]
    #[serial(skip_set)]
    fn skip_is_one_shot_per_mark() {
        clear_for_test();
        let h = unique_hash();
        mark_emitted(h);
        assert!(should_skip(h), "first check should skip");
        assert!(
            !should_skip(h),
            "second check should not skip — entry consumed"
        );
    }

    #[test]
    #[serial(skip_set)]
    fn two_unrelated_hashes_do_not_interfere() {
        clear_for_test();
        let a = unique_hash();
        let b = unique_hash().wrapping_add(1);
        mark_emitted(a);
        mark_emitted(b);
        assert!(should_skip(a));
        assert!(should_skip(b));
        assert!(!should_skip(a)); // both consumed
        assert!(!should_skip(b));
    }

    #[test]
    #[serial(skip_set)]
    fn expired_entry_does_not_skip() {
        clear_for_test();
        let h = unique_hash();
        // Manually inject an entry with a stale timestamp.
        {
            let mut s = set().lock().unwrap();
            s.push((h, Instant::now() - SKIP_TTL - Duration::from_secs(1)));
        }
        assert!(
            !should_skip(h),
            "expired entry should be pruned + not match"
        );
    }

    #[test]
    #[serial(skip_set)]
    fn prune_removes_expired_entries() {
        clear_for_test();
        // Insert a mix of fresh + expired entries.
        {
            let mut s = set().lock().unwrap();
            s.push((1u64, Instant::now())); // fresh
            s.push((2u64, Instant::now() - SKIP_TTL - Duration::from_secs(1))); // expired
            s.push((3u64, Instant::now())); // fresh
        }
        // Trigger a prune via should_skip on an unrelated hash.
        let _ = should_skip(99u64);
        let s = set().lock().unwrap();
        assert_eq!(s.len(), 2, "expired entry should have been pruned");
        let hashes: Vec<u64> = s.iter().map(|(h, _)| *h).collect();
        assert!(hashes.contains(&1));
        assert!(hashes.contains(&3));
        assert!(!hashes.contains(&2));
    }

    #[test]
    fn hash_content_deterministic() {
        // Pure helper, no global state. Doesn't need serial.
        let a = hash_content(b"hello world");
        let b = hash_content(b"hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_content_different_inputs_differ() {
        let a = hash_content(b"hello");
        let b = hash_content(b"world");
        assert_ne!(a, b);
    }

    #[test]
    #[serial(skip_set)]
    fn concurrent_mark_and_check_safe() {
        clear_for_test();
        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let h = i as u64;
                    mark_emitted(h);
                    assert!(should_skip(h), "thread {i} should match its own mark");
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
}
