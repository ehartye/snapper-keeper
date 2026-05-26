// Integration tests gated until T12 rewrites them against the native backend.
// Sidecar is downgraded to a private module in T5 (lib.rs), so the previous
// `snk_ocr::sidecar::run_tesseract` test calls would no longer compile from
// outside the crate. T12 replaces this file entirely with backend-trait tests.
#![cfg(any())]
