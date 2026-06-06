# Updater security model

This documents the auto-updater's threat model, the protections in place, and
the operator runbook for pulling a bad release. It is the written half of
issue #27.

## What the updater does

On launch (≈5s in) and every 24h, the app fetches a signed release manifest
(`latest.json`) from GitHub Releases, and if a newer version is offered,
downloads and installs the platform artifact. The flow lives in
`crates/snk-updater/src/plugin.rs`.

## Protections

### 1. Artifact signature (Tauri, pre-existing)

Every release artifact (`*-setup.exe`, `*.app.tar.gz`) is signed with minisign
in the release pipeline, and the signature is embedded in `latest.json`. Tauri's
updater verifies that signature against the public key pinned in
`app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) **before** installing.

**Consequence:** an attacker who tampers with the manifest or serves a malicious
binary cannot produce a valid signature without the private signing key, so
arbitrary-code injection through the update channel is blocked at the artifact
level. This is the load-bearing protection.

### 2. Downgrade floor (#27, this cluster)

The app persists the highest version it has ever been offered
(`updater.highest_seen_version` in the settings store). An offer **below** the
floor — the greater of that high-water mark and the running version — is refused
with `SuppressedByPolicy { DowngradeBlocked }` and surfaced in Settings → About.

This defends against a **rollback / pinning** attack: an adversary with control
of the network path serving a stale-but-validly-signed `latest.json` to pin a
user on a known-vulnerable older release. Tauri already refuses anything at or
below the *currently running* version; the floor extends that to the newest
version ever seen, not just the one installed.

Users who genuinely need to install an older build can toggle **Settings →
Updates → Allow rollback to older versions** (`updater.allow_rollback`, off by
default).

### 3. Signature failure is terminal (#27, this cluster)

A signature-verification error (`tauri_plugin_updater::Error::Minisign` and
related) is treated as **terminal for the process lifetime**: the updater
transitions to `RejectedBySignature`, a one-way state that disables further
update attempts and is shown in Settings → About. A bad signature means the
channel is compromised or misconfigured, so we stop rather than retry against a
hostile or broken endpoint. Network/IO errors remain retryable.

## Deliberately out of scope: signing `latest.json` itself

We do **not** publish a detached signature of the manifest and verify it
ourselves. Rationale:

- The thing that matters — the installed binary — is already signature-verified
  by Tauri (protection #1). A doctored manifest cannot yield a valid artifact
  signature without the private key.
- The only residual gain from manifest-signing is blocking rollback and
  version-string spoofing. Rollback is already closed by the downgrade floor
  (protection #2); version spoofing is cosmetic.
- Tauri fetches the manifest internally with no clean hook to verify a detached
  signature first, so implementing it means custom fetch/verify code that
  duplicates a guarantee we already have for the part that matters.

It also yields **zero** app-store/notarization benefit: the updater is compiled
out of the `store-edition` build, and Apple notarization is an automated
malware/hardened-runtime scan, not a review of update-manifest crypto.

## Operator runbook: pulling a bad release (kill switch)

There is no server-side "disable" button; mitigation is forward-only, which the
downgrade floor makes safe (clients won't accept a rollback to the bad build
once a newer one exists).

**To stop a bad release `vX` from spreading:**

1. **Publish a superseding release `vX+1`** containing the fix (or a revert).
   Because the offered version is higher, every client auto-updates *past* the
   bad build on its next 24h check. This is the primary mechanism.
2. **Edit the GitHub Release for `vX`** to remove its artifacts / mark it as a
   pre-release, so fresh installs and manual downloads stop receiving it. (This
   does not retroactively pull it from clients already on `vX`; step 1 does.)
3. **Never re-tag a lower version to "fix forward".** The downgrade floor will
   cause clients that already saw `vX` to *reject* it. Always go up.
4. If the signing key is suspected compromised, treat it as a key-rotation
   incident: generate a new keypair, update `plugins.updater.pubkey`, and ship a
   release signed with the new key. Clients on the old key will land in
   `RejectedBySignature` rather than silently accepting forged updates.

`force_min_version` (a manifest field that would hard-block running below a set
version) is **not** implemented; the superseding-release mechanism above covers
the same operational need without adding an unsigned manifest field for an
attacker to manipulate.
