# Release Process

End-to-end procedure for shipping a new version of snapper-keeper. Designed for the single-maintainer, no-deadline-pressure workflow this project actually runs in. Companion to [`docs/release-signing.md`](../release-signing.md) which covers signing-key setup.

## TL;DR

```
canary tag (prerelease: true)  →  self-install ~1-2 weeks  →  full upgrade cycle from N-1 to N proves the updater works  →  promote to stable tag (prerelease: false)
```

Two tags per release. The canary tag exercises the release pipeline, the auto-updater, and your daily flow before any other potential user sees the version as "latest."

## Why canary

The release pipeline is the most fragile path in the project — Tauri's signing, Apple notarization, Windows Trusted Signing, the updater's `latest.json` format, the GitHub Release artifact naming, all interact in ways that have surfaced bugs across Phase 7 (see `MEMORY.md` references). Many of those bugs were caught by the auto-updater itself failing, not by builds failing.

Canary surfaces auto-updater failures BEFORE the version is the default "latest" pointer. If the updater can't upgrade `vN-1 → vN-canary`, it definitely can't upgrade `vN-1 → vN`, and you've caught it without dragging other potential users along.

## Prerequisites

Before tagging anything:

1. **Branch protection clean**: main has no in-flight merges that might conflict with what's about to ship.
2. **Release-readiness manifest** (see [#44](https://github.com/ehartye/snapper-keeper/issues/44) when landed): freshly committed within the last 24h. Until #44 ships, run the equivalent checks manually:
   - `cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater`
   - `pnpm --filter @snk/app exec vitest run`
   - `pnpm lint && pnpm typecheck`
   - `cargo audit` once #46 is wired
3. **Signing infrastructure healthy**: see [`docs/release-signing.md`](../release-signing.md) for Tauri Ed25519 + macOS Developer ID + Azure Artifact Signing setup. Verify the secrets are still valid (Azure SP credentials in particular have expiration dates).
4. **Manual smoke checklist** (if the feature being released is destructive-action-sensitive): run the relevant `docs/superpowers/manual-checklist-*.md` against a dev build before tagging.

## Canary phase

### 1. Cut the canary tag

Version semantics: append `-rc.N` or `-canary.N` to the next stable target. Tauri's updater treats anything matching SemVer pre-release identifiers as a prerelease — it won't auto-offer the canary to clients on stable channels.

```bash
# Example: cutting 0.2.0's first canary
git tag v0.2.0-rc.1
git push origin v0.2.0-rc.1
```

The `release.yml` workflow runs on tag-push. It builds, signs, and creates a GitHub Release. **Verify `prerelease: true`** on the resulting release — the workflow may auto-detect prerelease from SemVer, but check the GitHub UI shows "Pre-release" badge.

### 2. Self-install the canary

Download the appropriate installer from the canary's GitHub Release page and install over your existing `vN-1` build. **Watch for**:

- **Installer integrity**: signature warning prompts, SmartScreen warnings, Gatekeeper rejections.
- **Auto-updater pickup**: the existing `vN-1` snapper-keeper checks for updates on a periodic timer (currently 24h). Within 24h of canary publication, the updater should NOT offer the canary (because it's `prerelease: true`). Verify it doesn't.
- **First-launch behavior**: data dir migration, settings preservation, hotkey re-registration, tray icon presence.

### 3. Soak period (~1-2 weeks)

Use the canary daily. Watch for:

- Crashes (`%LOCALAPPDATA%\com.snapper-keeper.app\logs\` on Windows, `~/Library/Logs/com.snapper-keeper.app/` on macOS — file logging from [#25](https://github.com/ehartye/snapper-keeper/issues/25) when landed)
- Memory growth (snapper-keeper is long-running; leaks compound)
- Background watcher regressions (clipboard, OCR queue, hotkey listener)
- Auto-updater periodic check failures (look for `update check failed` log entries)

If anything breaks: file an issue, fix on main, cut a new canary `vN-rc.2`. Repeat until a canary runs for a week clean.

### 4. Proof of upgrade

Before promoting to stable, you MUST exercise the auto-updater path from `vN-1 → vN-rc.<latest>`:

1. Install `vN-1` from its GitHub Release (downgrade if currently on canary).
2. Wait for the periodic update check OR force one (eventual "Check Now" button per [#36](https://github.com/ehartye/snapper-keeper/issues/36)).
3. Confirm the updater finds NOTHING (because canary is prerelease, stable channel ignores it). This is the test — silence here means the prerelease gate works.
4. Manually install the canary on top of `vN-1`.
5. Confirm successful upgrade: app version reads `vN-rc.X`, settings preserved, library DB intact (V005+ migrations applied cleanly).

This step catches updater regressions that only surface on actual update — not first-install.

## Stable promotion

### 1. Cut the stable tag

```bash
git tag v0.2.0
git push origin v0.2.0
```

The workflow runs again. **Verify `prerelease: false`** on the resulting release.

### 2. Verify `latest.json` points at the stable tag

```bash
# Windows
curl -L https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json | jq

# Check `version` field matches v0.2.0
```

This is the URL `tauri.conf.json`'s `plugins.updater.endpoints[0]` resolves to. If it points at the canary instead of the stable, GitHub's "latest release" detection picked the wrong one (which happens if the canary release wasn't actually marked prerelease).

### 3. Stable upgrade proof (different from canary upgrade proof)

Install a fresh `vN-1` (or downgrade). Wait for the periodic check. The updater SHOULD now offer `vN`. Accept the upgrade. Confirm the app restarts on `vN`.

This is the final gate. If this fails, the release is broken even though all CI passed.

## Rollback

If a stable release ships broken (the canary process should have caught this, but it can fail):

### 1. Mark the bad release as prerelease

Via GitHub UI: Edit release → Set as a pre-release → Update. This causes the auto-updater to stop offering it to stable-channel users.

### 2. Republish the prior stable as `latest`

GitHub's "latest release" detection picks the highest non-prerelease tag. After step 1, this should be `vN-1` again automatically.

Verify:

```bash
curl -L https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json | jq .version
```

Should show `vN-1`.

### 3. Communicate

Open a tracking issue describing the regression. If anyone other than you is using the app, post in whatever channel they'd see.

### 4. Cut a fix release as `vN+1`

Don't try to patch `vN` in place — auto-updaters detect by version number, and a "fixed vN" would have the same version as the broken one. Cut `vN.Y+1` (where Y is the patch component) or `vN+1` and go back through the canary process for that fix.

## Anti-patterns

- **Tag-and-pray**: cutting a stable tag without canary soak. The auto-updater is the project's main durability surface for installed users; verifying it works is non-optional.
- **Force-pushing a tag**: never `git push --force` a release tag. The auto-updater hashes the binaries at the tag — if you replace the binary under a tag, clients that already downloaded it have a mismatch. Always cut a new tag.
- **Skipping the upgrade proof**: a fresh install of `vN` proves the installer works; only the `vN-1 → vN` path proves the UPDATER works. Both are required.
- **Promoting a canary without exercising it**: spending two weeks NOT using the canary is the same as no canary. The soak period only works if you're actually catching bugs in it.

## Cadence

Targeted cadence (subject to no-pressure framing — when something's worth shipping, ship it; otherwise no rush):

- Canary cuts: when a meaningful feature or fix has landed on main and CI is fully green
- Stable promotion: when the most recent canary has soaked ~1-2 weeks without surfacing issues
- Patch releases (e.g. `v0.2.1`): on the same canary-then-stable cycle, just compressed if the fix is small and high-confidence

There's no calendar cadence. The project ships when it's ready.
