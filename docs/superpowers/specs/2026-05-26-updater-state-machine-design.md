# Updater State Machine — Unified Contract

Date: 2026-05-26  
Status: Approved for implementation

Source issues:

- [M-U10 synthesis note](../reviews/2026-05-24-prerelease/synthesis.md)
- [#27 downgrade floor + signed latest.json](https://github.com/ehartye/snapper-keeper/issues/27)
- [#51 updater state expansion](https://github.com/ehartye/snapper-keeper/issues/51)
- [#23 updater privacy/policy suppression (implementable subset)](https://github.com/ehartye/snapper-keeper/issues/23)

## 1) Goal

Define a single updater state contract before wiring #27/#23 behavior so future PRs do not bolt on parallel flags or collide on enum shape.

## 2) Canonical state set

```rust
enum UpdateStatus {
    Idle,
    Checking,
    Available { version: String, urgency: Urgency },
    Downloading { progress: f32 },
    Ready { version: String },
    Installing,
    Error { reason: UpdateErrorKind, retryable: bool },
    RejectedBySignature,
    SuppressedByPolicy { reason: SuppressionReason },
    Skipped { until_epoch_ms: i64 },
}
```

IPC note: the enum is internally-tagged with `"kind"` via serde, so the `Error` variant uses `reason` (not `kind`) for its error field to avoid a tag/field name collision. `Skipped` stores the defer instant as epoch-ms (`until_epoch_ms: i64`) rather than `Instant` so it round-trips cross-process safely.

## 3) Meaning of each state

- `Idle`: steady state; no active check/download/install.
- `Checking`: update discovery in progress.
- `Available`: update discovered; urgency included for UI prioritization.
- `Downloading`: background asset fetch/install prep with progress percent.
- `Ready`: payload downloaded and staged; restart can apply.
- `Installing`: restart/apply path has started.
- `Error`: transient or terminal failure metadata; `retryable` controls scheduler behavior.
- `RejectedBySignature`: signature verification failed; terminal for process lifetime.
- `SuppressedByPolicy`: updater intentionally disabled (user setting, store edition, downgrade floor, etc.).
- `Skipped`: user deferred update checks/downloads until a future instant.

## 4) Required invariants

1. `RejectedBySignature` is terminal for the process lifetime (no transitions out).
2. `SuppressedByPolicy` blocks update checks until policy reason clears.
3. `Error { retryable: true }` permits scheduler retry; `retryable: false` requires explicit user/policy change.
4. `Installing` is transient and only entered from `Ready`.

## 5) Transition sketch

- `Idle -> Checking`
- `Checking -> Available | Idle | Error | SuppressedByPolicy | RejectedBySignature`
- `Available -> Downloading | Skipped | SuppressedByPolicy`
- `Downloading -> Ready | Error | SuppressedByPolicy | RejectedBySignature`
- `Ready -> Installing | Skipped`
- `Installing -> Idle | Error`
- `Error -> Checking | SuppressedByPolicy` (retry or policy toggle)
- `Skipped -> Checking` (once `until_epoch_ms` elapsed or user forces check)
- `SuppressedByPolicy -> Idle` (policy reason removed)
- `RejectedBySignature -> RejectedBySignature` only

## 6) Bundled implementation order

1. Land this state contract in Rust + TS updater bindings and UI formatter.
2. Wire #27 outcomes onto this contract:
   - signature failure => `RejectedBySignature`
   - downgrade-floor violation => `SuppressedByPolicy { reason: DowngradeBlocked }`
3. Wire #23 implementable policy outcomes:
   - user toggle off => `SuppressedByPolicy { reason: UserDisabled }`
   - store edition => `SuppressedByPolicy { reason: StoreEdition }`

