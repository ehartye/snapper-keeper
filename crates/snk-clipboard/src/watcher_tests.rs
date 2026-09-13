use super::*;
use crate::sensitivity::FakeProbe;
use crate::source_app::{SourceApp, SourceAppKind};
use serde_json::json;
use snk_library::settings;

fn fresh_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sk.db");
    let db = Db::open(&path).unwrap();
    (dir, db)
}

#[test]
fn sensitive_flag_skips_without_persisting() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let result = worker_step(
        ClipboardEvent::Text("secret".into()),
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: true },
        None,
    );
    assert_eq!(result, StepResult::Skipped(SkipReason::SensitiveFlag));
    assert!(state.last_hash.is_some(), "last_hash should be set on skip");

    let items =
        snk_library::clipboard::list(&db, snk_library::ListClipboardQuery::default()).unwrap();
    assert_eq!(
        items.len(),
        0,
        "no row should be inserted on sensitive skip"
    );
}

#[test]
fn blocked_app_skips_without_persisting() {
    let (tmp, db) = fresh_db();
    settings::set(
        &db,
        "clipboard.app_blocklist",
        &json!([{
            "identifier": "1password.exe",
            "display_name": "1Password",
            "kind": "windows_exe"
        }]),
    )
    .unwrap();
    let src = SourceApp {
        identifier: "1password.exe".into(),
        display_name: "1Password".into(),
        kind: SourceAppKind::WindowsExe,
    };
    let mut state = WatcherState::new();
    let result = worker_step(
        ClipboardEvent::Text("password123".into()),
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        Some(src.clone()),
    );
    assert_eq!(
        result,
        StepResult::Skipped(SkipReason::AppBlocked(src.identifier))
    );
    let items =
        snk_library::clipboard::list(&db, snk_library::ListClipboardQuery::default()).unwrap();
    assert_eq!(items.len(), 0);
}

#[test]
fn allowed_text_event_is_saved_with_source_app() {
    let (tmp, db) = fresh_db();
    let src = SourceApp {
        identifier: "code.exe".into(),
        display_name: "Visual Studio Code".into(),
        kind: SourceAppKind::WindowsExe,
    };
    let mut state = WatcherState::new();
    let result = worker_step(
        ClipboardEvent::Text("hello".into()),
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        Some(src.clone()),
    );
    match result {
        StepResult::Saved { item_id } => {
            let stored = snk_library::clipboard::get(&db, &item_id).unwrap();
            assert_eq!(stored.source_app, Some(src.identifier));
        }
        other => panic!("expected Saved, got {other:?}"),
    }
}

#[test]
fn duplicate_hash_skips_without_re_inserting() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let probe = FakeProbe { answer: false };

    let first = worker_step(
        ClipboardEvent::Text("dup".into()),
        &mut state,
        &db,
        tmp.path(),
        &probe,
        None,
    );
    assert!(matches!(first, StepResult::Saved { .. }));

    let second = worker_step(
        ClipboardEvent::Text("dup".into()),
        &mut state,
        &db,
        tmp.path(),
        &probe,
        None,
    );
    assert_eq!(second, StepResult::Skipped(SkipReason::DuplicateHash));
}

#[test]
fn empty_text_is_skipped() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let result = worker_step(
        ClipboardEvent::Text(String::new()),
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        None,
    );
    assert_eq!(result, StepResult::Skipped(SkipReason::EmptyContent));
}

/// Build a minimal 2×2 RGBA byte vec (4 bytes per pixel, solid red).
fn red_2x2_rgba() -> Vec<u8> {
    [255, 0, 0, 255].repeat(4) // 16 bytes total: 4 pixels × 4 bytes/pixel (RGBA)
}

#[test]
fn encode_rgba_to_png_produces_valid_png() {
    let rgba = red_2x2_rgba();
    let png = encode_rgba_to_png(&rgba, 2, 2).expect("encode should succeed");
    // PNG magic bytes: 0x89 50 4E 47 0D 0A 1A 0A
    assert_eq!(
        &png[0..8],
        b"\x89PNG\r\n\x1a\n",
        "should start with PNG header"
    );
}

#[test]
fn encode_then_decode_round_trips_pixels() {
    let rgba = red_2x2_rgba();
    let png = encode_rgba_to_png(&rgba, 2, 2).expect("encode");
    let img = image::load_from_memory(&png).expect("decode");
    let decoded = img.to_rgba8().into_raw();
    assert_eq!(decoded, rgba, "decoded pixels must match original RGBA");
}

#[test]
fn encode_rgba_to_png_rejects_mismatched_dimensions() {
    // 3 bytes cannot form a 2×2 RGBA image (needs 16 bytes).
    let result = encode_rgba_to_png(&[1, 2, 3], 2, 2);
    assert!(result.is_none(), "should return None for bad dimensions");
}

#[test]
fn image_event_is_saved_as_valid_png_on_disk() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let rgba = red_2x2_rgba();
    let result = worker_step(
        ClipboardEvent::Image {
            bytes: rgba,
            width: 2,
            height: 2,
        },
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        None,
    );
    let item_id = match result {
        StepResult::Saved { item_id } => item_id,
        other => panic!("expected Saved, got {other:?}"),
    };

    let item = snk_library::clipboard::get(&db, &item_id).unwrap();
    assert_eq!(item.kind, snk_library::clipboard::ClipboardItemKind::Image);

    // The file must exist and contain a valid PNG.
    let file_path = item.file_path.expect("image item must have file_path");
    let full = tmp.path().join(file_path);
    assert!(full.exists(), "PNG file should be on disk");
    let bytes = std::fs::read(&full).unwrap();
    assert_eq!(
        &bytes[0..8],
        b"\x89PNG\r\n\x1a\n",
        "stored file must be a PNG"
    );
}

#[test]
fn duplicate_image_event_deduplicates() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let rgba = red_2x2_rgba();

    let first = worker_step(
        ClipboardEvent::Image {
            bytes: rgba.clone(),
            width: 2,
            height: 2,
        },
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        None,
    );
    assert!(matches!(first, StepResult::Saved { .. }));

    // Same pixels again — should be skipped as DuplicateHash, not insert a second row.
    let second = worker_step(
        ClipboardEvent::Image {
            bytes: rgba,
            width: 2,
            height: 2,
        },
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        None,
    );
    assert_eq!(second, StepResult::Skipped(SkipReason::DuplicateHash));
}
#[test]
#[serial_test::serial]
fn own_write_remains_suppressed_after_one_shot_token_is_consumed() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let text = "own-write-generation-regression";
    skip_set::mark_emitted(skip_set::hash_content(text.as_bytes()));
    for expected in [SkipReason::OwnWrite, SkipReason::DuplicateHash] {
        assert_eq!(
            worker_step(
                ClipboardEvent::Text(text.into()),
                &mut state,
                &db,
                tmp.path(),
                &FakeProbe { answer: false },
                None
            ),
            StepResult::Skipped(expected)
        );
    }
    assert!(snk_library::clipboard::list(&db, Default::default())
        .unwrap()
        .is_empty());
    state.last_hash = None; // A new macOS generation permits a real external copy.
    assert!(matches!(
        worker_step(
            ClipboardEvent::Text(text.into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            None
        ),
        StepResult::Saved { .. }
    ));
}

#[test]
fn image_disk_failure_can_recover_without_changing_content() {
    let (tmp, db) = fresh_db();
    let invalid_root = tmp.path().join("not-a-directory");
    std::fs::write(&invalid_root, b"file").unwrap();
    let mut state = WatcherState::new();
    let event = || ClipboardEvent::Image {
        bytes: red_2x2_rgba(),
        width: 2,
        height: 2,
    };
    assert_eq!(
        worker_step(
            event(),
            &mut state,
            &db,
            &invalid_root,
            &FakeProbe { answer: false },
            None
        ),
        StepResult::Skipped(SkipReason::PersistFailed)
    );
    assert_eq!(state.last_hash, None);
    assert!(matches!(
        worker_step(
            event(),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            None
        ),
        StepResult::Saved { .. }
    ));
}

#[path = "watcher_persistence_tests.rs"]
mod persistence;
