# Release Pipeline Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Move all release-signing secrets out of `cargo build` env via a build/sign/publish job split, gate sign jobs behind a `production-release` environment with artifact verification, pin all CI actions by commit SHA, and wire up nightly supply-chain advisories + per-release CycloneDX SBOM.

**Architecture:** Three sequenced PRs. PR-1 is a mechanical SHA-pinning sweep (foundation; no functional change). PR-2 restructures `release.yml` into `verify-pubkey → build (matrix, no secrets) → artifact-verify → environment gate → sign-* (matrix, with secrets) → publish`, and adds a stub `snk-smoke-target` crate for the per-sign-job smoke roundtrip. PR-3 adds a scheduled `audit.yml` workflow (per-advisory issue lifecycle) and extends `release.yml` with CycloneDX SBOM generation + upload.

**Tech Stack:** GitHub Actions (YAML), Bash, PowerShell (Windows), Rust (`snk-smoke-target` stub + tooling: `cargo-audit`, `cargo-cyclonedx`), Node 20 (`@cyclonedx/cyclonedx-npm`, `@cyclonedx/cyclonedx-cli`, audit-sync script + tests), `minisign` CLI, `dotnet sign` CLI (Windows Azure Trusted Signing), `gh` CLI.

**Source spec:** [`docs/superpowers/specs/2026-05-24-release-pipeline-hardening-design.md`](../specs/2026-05-24-release-pipeline-hardening-design.md)

**Prerequisites (before PR-2 lands):** Eric creates `production-release` environment in repo Settings → Environments. Required reviewers: `ehartye`. Deployment branches and tags: Selected → Tags → `v*`. Wait timer: 0. No environment secrets. See spec §11.

---

## PR Overview

| PR | Branch | Scope | Estimated diff |
|---|---|---|---|
| **PR-1** | `ci/sha-pin-actions` | #30 — SHA-pin every `uses:` in `ci.yml` + `release.yml`; drop `--prerelease` from `dotnet sign`; pin Tesseract chocolatey + SHA256-verify; replace `softprops/action-gh-release` with inline `gh release create`. | ~80 line YAML diff, no logic change |
| **PR-2** | `ci/build-sign-split` | #29 + #28 + #75 — Restructure `release.yml` job graph. Add `scripts/verify-pubkey.sh` + `scripts/smoke-sign-roundtrip.sh`. Add `tests/fixtures/smoke-target/` stub crate. | ~300 line YAML diff + 2 scripts + 1 crate |
| **PR-3** | `ci/audit-and-sbom` | #46 — New `audit.yml` workflow + per-advisory issue lifecycle script + tests. Extend `release.yml` `publish` job with CycloneDX SBOM generation. | ~120 line YAML + ~200 line JS + tests |

---

## PR-1: SHA-pin actions + version-pin tools (#30)

**Branch:** `ci/sha-pin-actions` off latest `main`.

Goal: no functional change. Every `uses: org/action@vX` becomes `uses: org/action@<SHA> # vX`. Every `cargo install` / `dotnet tool install` / `choco install` pinned to a known version. `softprops/action-gh-release@v2` replaced by inline `gh release create`. Validates by running normal CI + one throwaway release tag.

---

### Task 1.1: Inventory all `uses:` and resolve each tag to a commit SHA

**Files:**
- Read: `.github/workflows/ci.yml`
- Read: `.github/workflows/release.yml`

**Step 1: List every action in use**

Run:

```bash
grep -hE '^\s+(- )?uses:' .github/workflows/*.yml | sort -u
```

Expected output (this is the current inventory):

```
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
      - uses: actions/setup-dotnet@v4
      - uses: actions/setup-node@v4
      - uses: actions/upload-artifact@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: pnpm/action-setup@v3
      - uses: softprops/action-gh-release@v2
      - uses: Swatinem/rust-cache@v2
```

**Step 2: For each action, resolve the floating tag to a commit SHA**

For each `org/repo@tag` pair, run this lookup (`actions/checkout@v4` example):

```bash
ORG=actions; REPO=checkout; TAG=v4
SHA=$(gh api repos/$ORG/$REPO/git/refs/tags/$TAG --jq '.object.sha')
TYPE=$(gh api repos/$ORG/$REPO/git/refs/tags/$TAG --jq '.object.type')
if [ "$TYPE" = "tag" ]; then
  # Annotated tag — follow indirection to the commit
  SHA=$(gh api repos/$ORG/$REPO/git/tags/$SHA --jq '.object.sha')
fi
echo "uses: $ORG/$REPO@$SHA # $TAG"
```

Repeat for each pair listed in Step 1. Note: `dtolnay/rust-toolchain@stable` AND `pnpm/action-setup@v3` are both branch aliases, not tags — use `gh api repos/$ORG/$REPO/git/refs/heads/$BRANCH --jq '.object.sha'` for those. (`pnpm/action-setup` only has explicit version tags like `v3.0.0`; `v3` is a maintainer-updated major-version alias branch.) If `gh api ...refs/tags/$TAG` returns an array instead of an object, that's the giveaway — the tag doesn't exist; check `git/refs/heads/$TAG` instead.

**Step 3: Write the resolved SHAs to a scratch file for the next tasks**

Run:

```bash
cat > /tmp/sha-pins.txt <<'EOF'
# Format: <org/repo>@<sha> # <tag>
# Filled in by Task 1.1 Step 2
EOF
# Append your resolved lines here
```

This file is reference-only; do not commit it.

---

### Task 1.2: Apply SHA pins to `ci.yml`

**Files:**
- Modify: `.github/workflows/ci.yml` (every `uses:` line)

**Step 1: Replace each `uses:` line with the SHA-pinned version**

For each `uses: org/repo@vX` line in `ci.yml`, replace with `uses: org/repo@<resolved-sha> # vX`. Example diff:

```diff
-      - uses: actions/checkout@v4
+      - uses: actions/checkout@<40-char-sha-from-Task-1.1> # v4
```

Apply to all lines. The floating-tag comment after the SHA is required for human readability.

**Step 2: Sanity-check via YAML parser**

Run:

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: no output, exit 0.

**Step 3: Stage and verify diff is mechanical-only**

Run:

```bash
git diff .github/workflows/ci.yml | head -60
```

Expected: every change is a `-uses: org/repo@vX` removal paired with a `+uses: org/repo@<sha> # vX` addition. No other changes.

---

### Task 1.3: Apply SHA pins to `release.yml`

**Files:**
- Modify: `.github/workflows/release.yml` (every `uses:` line EXCEPT `softprops/action-gh-release@v2` — that one gets replaced in Task 1.6)

**Step 1: Replace each `uses:` line with the SHA-pinned version**

Same pattern as Task 1.2. Skip the `softprops/action-gh-release@v2` line — it's removed entirely in Task 1.6.

**Step 2: YAML sanity check**

Run:

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

Expected: no output, exit 0.

---

### Task 1.4: Pin `dotnet sign` tool to a specific Microsoft prerelease version (no `--prerelease` flag, just an explicit `--version`)

**Files:**
- Modify: `.github/workflows/release.yml` (the "Install dotnet sign tool" step, currently around line 86–98)

> **Important — two-strike lesson from PR-1 throwaway runs:**
>
> 1. The Microsoft `sign` tool (https://github.com/dotnet/sign) has **never had a stable release** — it only ships as `0.9.1-beta.*` prereleases. The NuGet ID `sign` is *shared* with an unrelated library `Sign 1.x` by Rafał Jasica (assembly-signing library, NOT a .NET tool). So `dotnet tool install --global sign --version 1.x` fails with `Package sign is not a .NET tool.` because it picks Jasica's package — exactly what happened on v0.0.0-sha-pin-test-1.
> 2. `dotnet tool install` **rejects** the combination `--prerelease --version <X>` with `The --prerelease and --version options are not supported in the same command` — exactly what happened on v0.0.0-sha-pin-test-2.
>
> The working pattern is: **specify the prerelease version explicitly via `--version 0.9.1-beta.NNNNN.N` and omit `--prerelease`.** NuGet looks up that exact version (which only exists in Microsoft's package — Jasica's only has stable `1.x`), so the package-identity disambiguation happens naturally.

**Step 1: Find the latest Microsoft `sign` prerelease (must be a `DotnetTool` package by Microsoft)**

Run:

```bash
# All versions including prereleases
curl -s https://api.nuget.org/v3-flatcontainer/sign/index.json | jq -r '.versions[]' | tail -20

# For each candidate, confirm it's Microsoft + DotnetTool by inspecting the nuspec:
VER=0.9.1-beta.26227.3  # or whichever is newest from the list above
curl -s https://api.nuget.org/v3-flatcontainer/sign/$VER/sign.nuspec | grep -E '<authors>|packageType '
# Expect: <authors>Microsoft</authors>  and  <packageType name="DotnetTool" />
```

Record the latest Microsoft+DotnetTool version as `SIGN_VERSION`. Skip any stable-looking `1.x` entry — those are Jasica's package.

**Step 2: Replace the install command in `release.yml`**

Current:

```powershell
dotnet tool install --global --prerelease sign
```

Replace with (substituting `<SIGN_VERSION>` from Step 1):

```powershell
# Microsoft's sign tool only ships as 0.9.1-beta.* prereleases. We resolve
# the package-identity collision with Sign 1.x by Rafal Jasica (an
# unrelated library, NOT a .NET tool) by version-pinning explicitly: the
# beta version only exists in Microsoft's package. We can NOT use
# --prerelease alongside --version (dotnet rejects the combination).
dotnet tool install --global sign --version <SIGN_VERSION>
```

**Step 3: Verify the rest of the install step still emits the correct PATH config**

Read the surrounding context — the step also appends `$env:USERPROFILE\.dotnet\tools` to `$env:GITHUB_PATH`. Leave that alone.

---

### Task 1.5: Pin Tesseract chocolatey version + capture expected installer SHA256

**Files:**
- Modify: `.github/workflows/release.yml` (the "Bundle Tesseract (Windows)" step, currently around line 66–76)

> **Important:** The Tesseract chocolatey package bundles the installer **inside the .nupkg** (file `tools/tesseract-ocr-w64-setup-<VERSION>.exe`). It is **NSIS `.exe`**, not `.msi` — `4D 5A` PE signature, not `D0 CF` OLE compound. An earlier draft of this plan assumed MSI; that was wrong. Also: since the installer ships inside the .nupkg, the SHA256 can be extracted directly from the package — no local `choco install` required. The post-install hash check in the workflow is still useful as a defense-in-depth against a malicious version-pin that swaps the bundled executable.

**Step 1: Identify the current stable Tesseract chocolatey version**

Query the community feed (works on any OS with curl + a JSON parser; doesn't require choco installed):

```powershell
Invoke-RestMethod -Uri "https://community.chocolatey.org/api/v2/Packages()?`$filter=Id eq 'tesseract'&`$orderby=Published desc&`$top=5" |
  Select-Object @{n='Version';e={$_.properties.Version}}, @{n='IsLatestVersion';e={$_.properties.IsLatestVersion.'#text'}}
```

Record the `IsLatestVersion: true` row as `TESSERACT_VERSION`. As of the time this plan was written: `5.5.0.20241111`.

**Step 2: Compute the expected SHA256 of the bundled installer (.exe, NOT .msi)**

The installer ships inside the .nupkg. Download the package, extract the bundled `.exe`, hash it. This works on any OS — Windows shown:

```powershell
$VER = '<TESSERACT_VERSION>'
$tmp = New-Item -ItemType Directory -Force -Path "$env:TEMP\tesseract-nupkg-inspect" | ForEach-Object FullName
Invoke-WebRequest -Uri "https://community.chocolatey.org/api/v2/package/tesseract/$VER" -OutFile "$tmp\tesseract.nupkg" -UseBasicParsing
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead("$tmp\tesseract.nupkg")
$entry = $zip.Entries | Where-Object { $_.FullName -like 'tools/tesseract-ocr-w64-setup-*.exe' } | Select-Object -First 1
$out = [System.IO.File]::OpenWrite("$tmp\$($entry.Name)")
$entry.Open().CopyTo($out); $out.Close()
$zip.Dispose()
(Get-FileHash -Algorithm SHA256 "$tmp\$($entry.Name)").Hash
```

Record the hash as `TESSERACT_SHA256`. Both `<TESSERACT_VERSION>` and `<TESSERACT_SHA256>` go into the workflow as literals (Step 3). For `5.5.0.20241111` the SHA256 is `F3FC4236425B690C8BE756F35793F77394EE004BE0A6460A440C754D892F68BC`.

**Step 3: Replace the Bundle Tesseract step in `release.yml`**

Replace the existing step body with (substituting both literals):

```yaml
      - name: Bundle Tesseract (Windows)
        if: runner.os == 'Windows'
        shell: powershell
        env:
          TESSERACT_VERSION: '<TESSERACT_VERSION>'
          TESSERACT_SHA256: '<TESSERACT_SHA256>'
        run: |
          # --requirechecksums is a no-op for this package (the installer
          # ships inside the .nupkg rather than being fetched via
          # Get-ChocolateyWebFile), but kept as belt-and-suspenders in case
          # the package model changes upstream.
          choco install tesseract --version=$env:TESSERACT_VERSION --no-progress --confirm --requirechecksums

          # Tesseract's chocolatey package bundles an NSIS .exe (not .msi).
          $installer = (Get-ChildItem "$env:ChocolateyInstall\lib\tesseract" -Filter '*.exe' -Recurse | Select-Object -First 1).FullName
          if (-not $installer) {
            Write-Error "Tesseract installer .exe not found after install"
            exit 1
          }
          $actual = (Get-FileHash -Algorithm SHA256 $installer).Hash
          # PowerShell -ne is case-insensitive on strings.
          if ($actual -ne $env:TESSERACT_SHA256) {
            Write-Error "Tesseract installer SHA256 mismatch. Expected: $env:TESSERACT_SHA256. Got: $actual."
            exit 1
          }
          Write-Host "Tesseract installer SHA256 verified."

          $src = 'C:\Program Files\Tesseract-OCR'
          $dest = 'app\src-tauri\resources\tesseract'
          New-Item -ItemType Directory -Force -Path $dest | Out-Null
          Copy-Item -Path "$src\*" -Destination $dest -Recurse -Force
          Write-Host "Bundled tesseract:"
          Get-ChildItem $dest | Format-Table Name, Length
```

---

### Task 1.6: Replace `softprops/action-gh-release` with inline `gh release create`

**Files:**
- Modify: `.github/workflows/release.yml` (the "Create GitHub Release" step at the end of the `publish-release` job, currently around line 308–320)

**Step 1: Remove the `softprops/action-gh-release@v2` step entirely**

Delete the entire block:

```yaml
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          draft: false
          prerelease: false
          generate_release_notes: true
          files: |
            artifacts/**/*.dmg
            ...
```

**Step 2: Add the replacement inline step**

Add:

```yaml
      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          # `gh` is preinstalled on ubuntu-latest. Uses GITHUB_TOKEN automatically.
          gh release create "$GITHUB_REF_NAME" \
            --title "$GITHUB_REF_NAME" \
            --generate-notes \
            artifacts/**/*.dmg \
            artifacts/**/*.app.tar.gz \
            artifacts/**/*.app.tar.gz.sig \
            artifacts/**/*-setup.exe \
            artifacts/**/*-setup.exe.sig \
            latest.json
```

Note: the `--generate-notes` flag replaces `generate_release_notes: true` from the third-party action. The third-party action's `draft: false` and `prerelease: false` are the `gh` CLI defaults — no flags needed.

**Step 3: YAML sanity check**

Run:

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

Expected: no output, exit 0.

---

### Task 1.7: Local verification of CI workflow files

**Files:** none modified

**Step 1: Run repo-level lint and typecheck**

Run:

```bash
pnpm lint && pnpm typecheck
```

Expected: both pass.

**Step 2: Optionally run actionlint if installed**

Run:

```bash
which actionlint && actionlint .github/workflows/*.yml || echo "actionlint not installed (optional)"
```

If actionlint is installed and reports errors, fix them. If not installed, skip — CI will catch parser errors on push.

**Step 3: Commit the PR-1 changes**

Run:

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml
git status --short
```

Verify only the two YAML files are staged. Then commit:

```bash
git commit -m "$(cat <<'EOF'
ci: pin actions by SHA + pin Tesseract + replace softprops (#30)

- Every uses: org/action@vX in ci.yml and release.yml now uses the
  full commit SHA with a # vX comment for human readability.
- dotnet sign install no longer uses --prerelease; pinned to a specific
  stable version.
- Tesseract chocolatey package version-pinned + SHA256-verified after
  download.
- Replaced softprops/action-gh-release@v2 with inline `gh release create`
  so contents:write is held only by our own step, not a third-party action.

No functional change to the release pipeline. Validated by normal CI
plus one throwaway release-tag run.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 1.8: Push branch + open PR

**Step 1: Push the branch**

Run:

```bash
git push -u origin ci/sha-pin-actions
```

**Step 2: Open PR**

Run:

```bash
gh pr create --title "ci: pin actions by SHA + pin Tesseract chocolatey + replace softprops (#30)" --body "$(cat <<'EOF'
## Summary
- Pins every `uses:` in `ci.yml` and `release.yml` to a commit SHA, preserving the floating tag as a `# vX` comment.
- Pins `dotnet sign` tool to a specific Microsoft prerelease version (`0.9.1-beta.*` — Microsoft's tool only ships as prereleases under the `sign` NuGet ID; pinning explicitly disambiguates from `Sign 1.x` by Rafał Jasica, an unrelated library that is NOT a .NET tool).
- Pins Tesseract chocolatey package version + SHA256-verifies the bundled installer (NSIS `.exe`, not MSI as an earlier plan draft assumed).
- Replaces `softprops/action-gh-release@v2` with inline `gh release create`, so `contents:write` is held only by our own step.

Implements #30. Part of the [release-pipeline hardening cluster](../blob/main/docs/superpowers/specs/2026-05-24-release-pipeline-hardening-design.md).

## Test plan
- [ ] CI passes (`lint-typecheck`, `ts-test`, `rust-test`, `coverage`) — validates that every pinned SHA resolves to a real, reachable commit
- [ ] Push throwaway tag `v0.0.0-sha-pin-test-1` to this branch and confirm `release.yml` runs end-to-end (validates pinned versions of `dotnet sign`, Tesseract chocolatey, and inline `gh release create` all work)
- [ ] Delete throwaway tag + release after

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Capture the PR URL for the user.

---

### Task 1.9: Validate via throwaway release-pipeline test

**Step 1: Push throwaway tag**

Run (replace `v0.0.0-sha-pin-test-1` if a higher number is needed):

```bash
git tag v0.0.0-sha-pin-test-1
git push origin v0.0.0-sha-pin-test-1
```

**Step 2: Watch the release workflow**

Run:

```bash
gh run watch $(gh run list --workflow=release.yml --branch=ci/sha-pin-actions --limit=1 --json databaseId --jq '.[0].databaseId')
```

Expected: workflow runs to completion successfully across all three platforms (macOS-arm64, macOS-x64, Windows-x64) and creates a release `v0.0.0-sha-pin-test-1`.

If anything fails — most likely cause is a pinned SHA that doesn't resolve to the right commit, or a pinned Tesseract / dotnet-sign version that's no longer available. Fix the version/SHA, push again.

**Step 3: Clean up the test release and tag**

Run:

```bash
gh release delete v0.0.0-sha-pin-test-1 --yes --cleanup-tag
```

`--cleanup-tag` also deletes the tag from the remote. Verify locally:

```bash
git tag -d v0.0.0-sha-pin-test-1 2>/dev/null || true
git fetch --prune --prune-tags
```

**Step 4: Mark PR ready for merge**

If CI is green and the throwaway tag test succeeded, comment on the PR confirming successful throwaway-tag test (with the run URL). Then merge (or request review).

```bash
gh pr merge --squash --delete-branch
```

Wait for merge to complete, then `git checkout main && git pull` to sync.

---

## PR-2: build/sign split + verify-pubkey + smoke vehicle (#29 + #28 + #75)

**Branch:** `ci/build-sign-split` off latest `main` (after PR-1 merged).

**Prerequisite:** `production-release` environment exists in repo Settings → Environments with required reviewer = `ehartye` and deployment-branches rule `Tags v*`. If this is not set up, the sign jobs will hang indefinitely waiting for approval that can never be granted. Confirm with Eric before starting PR-2 implementation.

Goal: substantial restructure of `release.yml`. Add two scripts + one stub crate. Validate by two throwaway tag runs (reject the gate, then approve it).

---

### Task 2.1: Add `scripts/verify-pubkey.sh`

**Files:**
- Create: `scripts/verify-pubkey.sh`

**Step 1: Create the script**

Write the following to `scripts/verify-pubkey.sh`:

```bash
#!/usr/bin/env bash
# Fail loudly if TAURI_SIGNING_PRIVATE_KEY does not match the pubkey embedded
# in app/src-tauri/tauri.conf.json plugins.updater.pubkey.
#
# Approach: sign a known canary string with the private key, verify the
# signature with the embedded public key. If verify succeeds, the keys match.
# Tauri 2 has no `tauri signer sign --print-pub-key` flag, so the canary
# roundtrip is the robust pattern.
#
# Required env:
#   TAURI_SIGNING_PRIVATE_KEY          (base64 minisign private key)
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD (password used at key generation)
#
# Required tools on PATH: minisign, jq, base64.
#
# Side effects: writes/deletes temp files under TMPDIR. No network access.

set -euo pipefail

: "${TAURI_SIGNING_PRIVATE_KEY:?TAURI_SIGNING_PRIVATE_KEY must be set}"
: "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:?TAURI_SIGNING_PRIVATE_KEY_PASSWORD must be set}"

WORK=$(mktemp -d)
trap "rm -rf '$WORK'" EXIT

echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$WORK/priv.key"
echo "snapper-keeper-pubkey-drift-canary" > "$WORK/canary.txt"

# Sign the canary with the secret-held private key.
# minisign reads the password from stdin when -W is set or piped.
echo "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" | minisign -S -s "$WORK/priv.key" \
  -m "$WORK/canary.txt" -x "$WORK/canary.sig" >/dev/null

# Verify the canary with the embedded public key.
jq -r .plugins.updater.pubkey app/src-tauri/tauri.conf.json | base64 -d > "$WORK/pub.key"

if ! minisign -V -p "$WORK/pub.key" -m "$WORK/canary.txt" -x "$WORK/canary.sig" >/dev/null 2>&1; then
  echo "::error file=app/src-tauri/tauri.conf.json,line=109::Pubkey drift: TAURI_SIGNING_PRIVATE_KEY does not match plugins.updater.pubkey. Rotate one to match the other before tagging."
  exit 1
fi

echo "Pubkey OK: TAURI_SIGNING_PRIVATE_KEY matches tauri.conf.json plugins.updater.pubkey."
```

**Step 2: Make it executable**

Run:

```bash
chmod +x scripts/verify-pubkey.sh
```

**Step 3: Verify script syntax**

Run:

```bash
bash -n scripts/verify-pubkey.sh
```

Expected: no output, exit 0 (POSIX syntax check, no execution).

---

### Task 2.2: Create the `snk-smoke-target` stub crate

**Files:**
- Create: `tests/fixtures/smoke-target/Cargo.toml`
- Create: `tests/fixtures/smoke-target/src/main.rs`
- Create: `tests/fixtures/smoke-target/build.rs`
- Create: `tests/fixtures/smoke-target/smoke-target.exe.manifest`
- Create: `tests/fixtures/smoke-target/smoke-target.rc`
- Modify: `Cargo.toml` (workspace root — add to `[workspace.members]`)

> **Important:** RC.EXE (Microsoft Resource Compiler) does **not** accept side-by-side manifest XML as input. Passing the manifest directly to `embed_resource::compile(...)` fails with `RC2135 : file not found: encoding` (RC interprets the XML as resource declarations). The correct pattern is a thin `.rc` resource script that declares the manifest as resource type `RT_MANIFEST (24)`, id `1`; `embed-resource` feeds *that* to RC.EXE.

**Step 1: Create the Cargo manifest**

Write to `tests/fixtures/smoke-target/Cargo.toml`:

```toml
[package]
name = "snk-smoke-target"
version = "0.0.0"
edition = "2021"
publish = false
description = "Throwaway binary used by the release-pipeline smoke roundtrip — proves signing tools work before touching real artifacts."

[[bin]]
name = "snk-smoke-target"
path = "src/main.rs"

# Empty deps — keep this crate hermetic and zero-cost.
[dependencies]

# Windows-only manifest embedding via embed-resource. The build script
# feeds smoke-target.rc (a thin resource script) to RC.EXE, which
# embeds smoke-target.exe.manifest as RT_MANIFEST resource id 1.
[target.'cfg(windows)'.build-dependencies]
embed-resource = "3"
```

**Step 2: Create the binary source**

Write to `tests/fixtures/smoke-target/src/main.rs`:

```rust
fn main() {
    println!("snapper-keeper sign smoke ok");
}
```

**Step 3: Create the build script (Windows manifest embed)**

Write to `tests/fixtures/smoke-target/build.rs`:

```rust
fn main() {
    #[cfg(windows)]
    {
        // embed-resource invokes RC.EXE on the .rc resource script.
        // The .rc declares RT_MANIFEST id 1 pointing at the side-by-side
        // manifest XML, which RC.EXE then embeds into the linked .exe.
        // `let _` suppresses the unused-must-use warning on CompilationResult.
        let _ = embed_resource::compile("smoke-target.rc", embed_resource::NONE);
    }
}
```

**Step 4: Create the resource script (the indirection RC.EXE needs)**

Write to `tests/fixtures/smoke-target/smoke-target.rc`:

```rc
// RC.EXE doesn't accept side-by-side manifests directly; it expects a
// resource script. This file just declares resource id 1 of type
// RT_MANIFEST (24) pointing at the actual manifest XML.
1 24 "smoke-target.exe.manifest"
```

**Step 5: Create the Windows manifest**

Write to `tests/fixtures/smoke-target/smoke-target.exe.manifest`:

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
```

The `asInvoker` level is critical: without it, the Windows UAC heuristic (per `MEMORY.md` → "Windows UAC installer detection heuristic") would attempt to elevate this stub if its name happened to contain "update", "setup", or "install". The name `snk-smoke-target` was chosen to avoid those substrings; the manifest is belt-and-suspenders.

**Step 6: Add the crate to the workspace**

Read `Cargo.toml` (workspace root) and find the `[workspace]` section. Add `"tests/fixtures/smoke-target"` to the `members` array. Example diff (the exact existing entry list will vary):

```diff
 [workspace]
 members = [
     "app/src-tauri",
     "crates/snk-annotate",
     "crates/snk-capture",
     "crates/snk-clipboard",
     "crates/snk-hotkeys",
     "crates/snk-library",
     "crates/snk-ocr",
     "crates/snk-updater",
+    "tests/fixtures/smoke-target",
 ]
```

**Step 7: Verify the crate builds**

Run:

```bash
cargo build -p snk-smoke-target --release
```

Expected: clean build, produces `target/release/snk-smoke-target` (or `snk-smoke-target.exe` on Windows). On Linux/macOS the build.rs `#[cfg(windows)]` block is skipped, so `embed-resource` won't be a dependency hit there.

**Step 8: Verify the produced binary runs**

Run:

```bash
./target/release/snk-smoke-target
```

Expected output: `snapper-keeper sign smoke ok`

---

### Task 2.3: Add `scripts/smoke-sign-roundtrip.sh`

**Files:**
- Create: `scripts/smoke-sign-roundtrip.sh`

**Step 1: Create the script**

Write to `scripts/smoke-sign-roundtrip.sh`:

```bash
#!/usr/bin/env bash
# Smoke-test the platform signing toolchain against a throwaway stub binary
# BEFORE touching real release artifacts. If any step fails, the sign job
# aborts and no real artifact gets signed.
#
# Usage: smoke-sign-roundtrip.sh <windows|macos>
#
# Required env (varies by platform):
#   All:     TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD
#   Windows: AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET
#   macOS:   APPLE_SIGNING_IDENTITY (+ build.keychain unlocked beforehand)

set -euo pipefail

PLATFORM="${1:?usage: smoke-sign-roundtrip.sh <windows|macos>}"

# Cheap pubkey-diff first — verify-pubkey job already ran upstream, but it's
# cheap and fails fast before we burn Azure/Apple signing calls.
./scripts/verify-pubkey.sh

cargo build -p snk-smoke-target --release

case "$PLATFORM" in
  windows)
    cp target/release/snk-smoke-target.exe ./smoke.exe
    sign code artifact-signing \
      -ase https://eus.codesigning.azure.net \
      -asa HartyeTech \
      -ascp snapper-keeper \
      -v Information \
      ./smoke.exe
    signtool verify /pa /v ./smoke.exe
    ;;
  macos)
    cp target/release/snk-smoke-target ./smoke
    codesign --sign "$APPLE_SIGNING_IDENTITY" --options runtime --timestamp ./smoke
    codesign --verify --strict --verbose=2 ./smoke
    ;;
  *)
    echo "::error::Unknown platform: $PLATFORM"
    exit 2
    ;;
esac

rm -f ./smoke ./smoke.exe
echo "Smoke roundtrip passed for $PLATFORM."
```

**Step 2: Make it executable**

Run:

```bash
chmod +x scripts/smoke-sign-roundtrip.sh
```

**Step 3: Syntax check**

Run:

```bash
bash -n scripts/smoke-sign-roundtrip.sh
```

Expected: exit 0.

---

### Task 2.4: Commit the precursor — scripts + smoke-target crate

This commit lands before the `release.yml` restructure so the restructure commit's diff stays focused on YAML.

**Files staged:**
- `scripts/verify-pubkey.sh`
- `scripts/smoke-sign-roundtrip.sh`
- `tests/fixtures/smoke-target/Cargo.toml`
- `tests/fixtures/smoke-target/src/main.rs`
- `tests/fixtures/smoke-target/build.rs`
- `tests/fixtures/smoke-target/smoke-target.exe.manifest`
- `tests/fixtures/smoke-target/smoke-target.rc`
- `Cargo.toml` (workspace members)
- `Cargo.lock` (regenerated by `cargo build` in Task 2.2)

**Step 1: Stage**

Run:

```bash
git add scripts/verify-pubkey.sh scripts/smoke-sign-roundtrip.sh \
        tests/fixtures/smoke-target/Cargo.toml \
        tests/fixtures/smoke-target/src/main.rs \
        tests/fixtures/smoke-target/build.rs \
        tests/fixtures/smoke-target/smoke-target.exe.manifest \
        tests/fixtures/smoke-target/smoke-target.rc \
        Cargo.toml Cargo.lock
git status --short
```

Verify only those files are staged.

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore(ci): add verify-pubkey, smoke-sign-roundtrip scripts + smoke stub crate

Precursor for the release.yml build/sign split (#29). Adds the scripts and
the snk-smoke-target stub binary that the new sign jobs will consume:

- scripts/verify-pubkey.sh: sign-and-verify canary roundtrip that confirms
  TAURI_SIGNING_PRIVATE_KEY matches the pubkey embedded in tauri.conf.json
  (#28). Fails loudly if they have drifted.
- scripts/smoke-sign-roundtrip.sh: per-platform sign+verify roundtrip
  against a stub binary, run before any real artifact is signed (#75).
- tests/fixtures/smoke-target/: tiny "hello world" binary with Windows
  asInvoker manifest. Name avoids UAC-installer heuristic substrings.

No release.yml change in this commit; that's the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2.5: Restructure `release.yml` — add `verify-pubkey` job

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Install `minisign` on the verify-pubkey runner**

Add a new top-level job in `release.yml` before the existing matrix job. Insert this as the FIRST job under `jobs:`:

```yaml
  verify-pubkey:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@<sha> # v4   # use the SHA from PR-1 Task 1.1
      - name: Install minisign
        run: sudo apt-get update && sudo apt-get install -y minisign jq
      - name: Verify TAURI_SIGNING_PRIVATE_KEY matches tauri.conf.json pubkey
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: ./scripts/verify-pubkey.sh
```

Replace `<sha>` with the SHA for `actions/checkout@v4` from PR-1.

**Step 2: YAML sanity check**

Run:

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

Expected: exit 0.

---

### Task 2.6: Replace `build-and-release` matrix with secrets-free `build` matrix

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Replace the existing `build-and-release` job**

Delete the entire `build-and-release:` job (everything from `build-and-release:` down to but not including `publish-release:`). Replace with:

```yaml
  build:
    needs: verify-pubkey
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            label: macOS-arm64
            bundles: 'app'
          - os: macos-15-intel
            target: x86_64-apple-darwin
            label: macOS-x64
            bundles: 'app'
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            label: Windows-x64
            bundles: 'nsis'

    runs-on: ${{ matrix.os }}
    permissions:
      contents: read

    steps:
      - uses: actions/checkout@<sha> # v4
      - uses: pnpm/action-setup@<sha> # v3
        with:
          version: 9
      - uses: actions/setup-node@<sha> # v4
        with:
          node-version: 20
          cache: pnpm
      - uses: dtolnay/rust-toolchain@<sha> # stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@<sha> # v2
        with:
          key: ${{ matrix.target }}

      - run: pnpm install --frozen-lockfile

      - name: Bundle Tesseract (Windows)
        if: runner.os == 'Windows'
        shell: powershell
        env:
          TESSERACT_VERSION: '<TESSERACT_VERSION>'   # from PR-1 Task 1.5
          TESSERACT_SHA256: '<TESSERACT_SHA256>'    # from PR-1 Task 1.5
        run: |
          choco install tesseract --version=$env:TESSERACT_VERSION --no-progress --confirm --requirechecksums
          # Tesseract's chocolatey package bundles an NSIS .exe (not .msi).
          $installer = (Get-ChildItem "$env:ChocolateyInstall\lib\tesseract" -Filter '*.exe' -Recurse | Select-Object -First 1).FullName
          if (-not $installer) { Write-Error "Tesseract installer .exe not found"; exit 1 }
          $actual = (Get-FileHash -Algorithm SHA256 $installer).Hash
          if ($actual -ne $env:TESSERACT_SHA256) {
            Write-Error "Tesseract installer SHA256 mismatch. Expected: $env:TESSERACT_SHA256. Got: $actual."
            exit 1
          }
          $src = 'C:\Program Files\Tesseract-OCR'
          $dest = 'app\src-tauri\resources\tesseract'
          New-Item -ItemType Directory -Force -Path $dest | Out-Null
          Copy-Item -Path "$src\*" -Destination $dest -Recurse -Force

      # Windows: delete bundle.windows.signCommand from tauri.conf.json
      # before building. The build job is secrets-free and cannot invoke
      # `dotnet sign`. Attempts to disable the signCommand via --config
      # overlay don't work cleanly:
      #   - `signCommand: ""` -> Tauri tries to exec the empty command and
      #     fails with "program path has no file name".
      #   - `signCommand: null` -> unverified at time of writing; jq -d is
      #     the robust fix that just removes the field entirely.
      # The sign-win-x64 job runs `dotnet sign` against the unsigned
      # installer instead.
      - name: Strip signCommand from tauri.conf.json (Windows build)
        if: runner.os == 'Windows'
        shell: bash
        run: |
          jq 'del(.bundle.windows.signCommand)' app/src-tauri/tauri.conf.json > /tmp/conf.json
          mv /tmp/conf.json app/src-tauri/tauri.conf.json

      # Build with NO signing secrets in env.
      #
      # `createUpdaterArtifacts: false` is critical here. An earlier draft of
      # this plan overrode `plugins.updater.pubkey` to "" hoping Tauri would
      # interpret that as "no pubkey, skip updater minisign". It does not:
      # Tauri sees ANY pubkey value (including "") as "pubkey configured",
      # then errors with "A public key has been found, but no private key.
      # Make sure to set TAURI_SIGNING_PRIVATE_KEY environment variable."
      # — exactly because we INTENTIONALLY haven't set that env var. Setting
      # createUpdaterArtifacts: false skips the updater bundler entirely,
      # which is what we want: the sign jobs recreate the updater payload
      # (tar of signed .app + minisign on macOS; minisign on signed installer
      # on Windows) from the platform-signed artifacts.
      #
      # `shell: bash` is required: on windows-latest the default shell is
      # PowerShell, which tokenizes the inline JSON --config argument
      # differently and breaks the bundler. Git Bash is preinstalled on
      # Windows runners.
      - name: Build Tauri app (unsigned)
        shell: bash
        run: |
          pnpm tauri build --target ${{ matrix.target }} --bundles ${{ matrix.bundles }} \
            --config '{"bundle":{"createUpdaterArtifacts":false}}'

      # macOS: upload the .app directory. The sign job will codesign the .app,
      # then rebuild .dmg from the signed .app, tar the signed .app to make
      # .app.tar.gz, then minisign the tar.
      #
      # IMPORTANT: path glob is `macos/**`, NOT `macos/Snapper Keeper.app/**`.
      # upload-artifact uses the common ancestor of matched files as the
      # archive root. With `.app/**` the common ancestor is the .app
      # directory itself, which strips `Snapper Keeper.app/` from the
      # archive — the sign job's `"artifacts/unsigned/Snapper Keeper.app"`
      # reference would not exist after download. Uploading from `macos/**`
      # preserves `Snapper Keeper.app/...` as a subdirectory (and
      # incidentally also picks up the unsigned `.app.tar.gz` that the
      # updater bundler emits; the sign job ignores that and recreates it
      # post-codesign).
      - name: Upload unsigned macOS artifacts
        if: runner.os == 'macOS'
        uses: actions/upload-artifact@<sha> # v4
        with:
          name: unsigned-artifacts-${{ matrix.label }}
          path: |
            target/${{ matrix.target }}/release/bundle/macos/**
          if-no-files-found: error

      - name: Upload unsigned Windows artifacts
        if: runner.os == 'Windows'
        uses: actions/upload-artifact@<sha> # v4
        with:
          name: unsigned-artifacts-${{ matrix.label }}
          path: |
            target/${{ matrix.target }}/release/bundle/nsis/*-setup.exe
          if-no-files-found: error
```

Replace each `<sha>` placeholder with the SHA from PR-1's resolution. Replace the Tesseract literals with the values from PR-1 Task 1.5.

**Step 2: Plan-time validation of `--bundles` flag**

The spec flagged `--bundles app,updater` syntax as plan-time-validated. Confirm:

Run on any platform:

```bash
pnpm tauri build --help | grep -A 2 -- '--bundles'
```

Expected: shows `--bundles <BUNDLES>...` with examples or accepted comma-separated form. If the flag is `--bundle` (singular) or requires repeated flags (`--bundles app --bundles updater`), adjust the YAML accordingly. As of Tauri 2.5.x the flag accepts comma-separated values.

**Step 3: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

---

### Task 2.7: Add `artifact-verify` job

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Add the artifact-verify job after the `build` job, before `publish-release`**

Insert:

```yaml
  artifact-verify:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - name: Download all unsigned artifacts
        uses: actions/download-artifact@<sha> # v4
        with:
          path: artifacts
          pattern: unsigned-artifacts-*

      - name: Summarize artifacts
        run: |
          echo "## Unsigned build artifacts" >> "$GITHUB_STEP_SUMMARY"
          echo "" >> "$GITHUB_STEP_SUMMARY"
          echo "| Platform | File | Size (bytes) | SHA-256 |" >> "$GITHUB_STEP_SUMMARY"
          echo "|---|---:|---:|---|" >> "$GITHUB_STEP_SUMMARY"

          find artifacts -type f | sort | while read -r f; do
            # Skip Mac .app directory contents inside Info.plist etc.; report
            # only top-level installer-shaped files.
            case "$f" in
              *.exe|*.dmg|*.app.tar.gz|*.app/Contents/MacOS/*) ;;
              *) continue ;;
            esac
            platform=$(echo "$f" | awk -F/ '{print $2}' | sed 's/^unsigned-artifacts-//')
            name=$(basename "$f")
            size=$(stat -c%s "$f")
            sha=$(sha256sum "$f" | awk '{print $1}')
            printf "| %s | %s | %s | %s |\n" "$platform" "$name" "$size" "$sha" >> "$GITHUB_STEP_SUMMARY"
          done

          echo "" >> "$GITHUB_STEP_SUMMARY"
          echo "Review this table before approving the production-release environment gate." >> "$GITHUB_STEP_SUMMARY"
```

Note: for macOS, the `unsigned-artifacts-macOS-*` upload is the `.app` directory tree, so the summary reports the main executable inside `Contents/MacOS/`. For Windows, the upload is the `-setup.exe` directly.

**Step 2: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

---

### Task 2.8: Add `sign-mac-arm`, `sign-mac-x64`, `sign-win-x64` jobs

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Add the macOS sign job (template — duplicated per arch)**

Insert after `artifact-verify`:

```yaml
  sign-mac-arm:
    needs: artifact-verify
    runs-on: macos-latest
    environment: production-release
    permissions:
      contents: read
    env:
      LABEL: macOS-arm64
      ARCH: aarch64
    steps:
      - uses: actions/checkout@<sha> # v4

      - uses: dtolnay/rust-toolchain@<sha> # stable
        with:
          targets: aarch64-apple-darwin

      - name: Install minisign, jq, create-dmg
        run: brew install minisign jq create-dmg

      - name: Download unsigned artifact
        uses: actions/download-artifact@<sha> # v4
        with:
          name: unsigned-artifacts-${{ env.LABEL }}
          path: artifacts/unsigned

      - name: Import Apple certificate
        env:
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        run: |
          echo "$APPLE_CERTIFICATE" | base64 --decode > certificate.p12
          security create-keychain -p actions build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p actions build.keychain
          # 2-hour keychain timeout (per MEMORY.md → macOS keychain timeout in CI).
          security set-keychain-settings -t 7200 build.keychain
          security import certificate.p12 -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
          security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k actions build.keychain
          rm certificate.p12

      - name: Smoke-test signing toolchain
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
        run: ./scripts/smoke-sign-roundtrip.sh macos

      - name: Codesign .app
        env:
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
        run: |
          APP_PATH="artifacts/unsigned/Snapper Keeper.app"
          codesign --sign "$APPLE_SIGNING_IDENTITY" --options runtime --timestamp --deep "$APP_PATH"
          codesign --verify --deep --strict --verbose=2 "$APP_PATH"

      - name: Rebuild .dmg from signed .app
        run: |
          mkdir -p artifacts/signed
          create-dmg \
            --volname "Snapper Keeper" \
            --window-size 600 400 \
            --icon-size 100 \
            "artifacts/signed/Snapper Keeper.dmg" \
            "artifacts/unsigned/Snapper Keeper.app"

      - name: Notarize .dmg
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: |
          DMG="artifacts/signed/Snapper Keeper.dmg"
          xcrun notarytool submit "$DMG" \
            --apple-id "$APPLE_ID" \
            --password "$APPLE_PASSWORD" \
            --team-id "$APPLE_TEAM_ID" \
            --wait
          xcrun stapler staple "$DMG"

      - name: Tar signed .app for updater payload
        run: |
          cd artifacts/unsigned
          tar -czf "../signed/Snapper.Keeper_${{ env.ARCH }}.app.tar.gz" "Snapper Keeper.app"

      - name: Minisign the updater payload
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: |
          WORK=$(mktemp -d)
          echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$WORK/priv.key"
          echo "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" | minisign -S -s "$WORK/priv.key" \
            -m "artifacts/signed/Snapper.Keeper_${{ env.ARCH }}.app.tar.gz" \
            -x "artifacts/signed/Snapper.Keeper_${{ env.ARCH }}.app.tar.gz.sig"
          rm -rf "$WORK"

      - name: Rename .dmg per-arch
        run: |
          cd artifacts/signed
          mv "Snapper Keeper.dmg" "Snapper.Keeper_${{ env.ARCH }}.dmg"

      - name: Upload signed artifacts
        uses: actions/upload-artifact@<sha> # v4
        with:
          name: signed-artifacts-${{ env.LABEL }}
          path: |
            artifacts/signed/*.dmg
            artifacts/signed/*.app.tar.gz
            artifacts/signed/*.app.tar.gz.sig
          if-no-files-found: error

      - name: Clean up certificates
        if: always()
        run: rm -f certificate.p12
```

**Step 2: Duplicate for `sign-mac-x64`**

Same shape with these changes:

- Job name: `sign-mac-x64`
- `runs-on: macos-15-intel`
- `rust-toolchain` `targets: x86_64-apple-darwin`
- `env: LABEL: macOS-x64`, `ARCH: x86_64`

**Step 3: Add the Windows sign job**

```yaml
  sign-win-x64:
    needs: artifact-verify
    runs-on: windows-latest
    environment: production-release
    permissions:
      contents: read
    env:
      LABEL: Windows-x64
    steps:
      - uses: actions/checkout@<sha> # v4

      - uses: dtolnay/rust-toolchain@<sha> # stable
        with:
          targets: x86_64-pc-windows-msvc

      - uses: actions/setup-dotnet@<sha> # v4
        with:
          dotnet-version: '9.x'

      - name: Install dotnet sign tool
        shell: powershell
        run: |
          dotnet tool install --global sign --version <SIGN_VERSION>   # from PR-1 Task 1.4 (do NOT add --prerelease; dotnet rejects it alongside --version)
          $toolsPath = "$env:USERPROFILE\.dotnet\tools"
          $toolsPath | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append

      - name: Install minisign
        shell: powershell
        run: choco install minisign --no-progress --confirm

      - name: Download unsigned artifact
        uses: actions/download-artifact@<sha> # v4
        with:
          name: unsigned-artifacts-${{ env.LABEL }}
          path: artifacts/unsigned

      - name: Smoke-test signing toolchain
        shell: bash
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
          AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
          AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
        run: ./scripts/smoke-sign-roundtrip.sh windows

      - name: Authenticode-sign the installer
        shell: powershell
        env:
          AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
          AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
          AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
        run: |
          $exe = (Get-ChildItem artifacts/unsigned -Filter '*-setup.exe' | Select-Object -First 1).FullName
          if (-not $exe) { Write-Error "No -setup.exe found"; exit 1 }
          New-Item -ItemType Directory -Force -Path artifacts/signed | Out-Null
          $signed = Join-Path 'artifacts/signed' (Split-Path -Leaf $exe)
          Copy-Item $exe $signed
          sign code artifact-signing `
            -ase https://eus.codesigning.azure.net `
            -asa HartyeTech `
            -ascp snapper-keeper `
            -v Information `
            $signed
          signtool verify /pa /v $signed

      - name: Minisign the installer
        shell: bash
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: |
          EXE=$(find artifacts/signed -name '*-setup.exe' | head -1)
          WORK=$(mktemp -d)
          echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$WORK/priv.key"
          echo "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" | minisign -S -s "$WORK/priv.key" \
            -m "$EXE" -x "$EXE.sig"
          rm -rf "$WORK"

      - name: Upload signed artifacts
        uses: actions/upload-artifact@<sha> # v4
        with:
          name: signed-artifacts-${{ env.LABEL }}
          path: |
            artifacts/signed/*-setup.exe
            artifacts/signed/*-setup.exe.sig
          if-no-files-found: error
```

Replace `<SIGN_VERSION>` with the value from PR-1 Task 1.4.

**Step 4: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

---

### Task 2.9: Update the `publish-release` job to consume signed artifacts

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Update the job's `needs:` clause**

Change from:

```yaml
  publish-release:
    needs: build-and-release
```

To:

```yaml
  publish-release:
    needs: [sign-mac-arm, sign-mac-x64, sign-win-x64]
```

**Step 2: Update the download-artifact step**

Change the artifact pattern from `unsigned-artifacts-*` (which doesn't exist for publish to consume) to `signed-artifacts-*`:

```yaml
      - name: Download all signed artifacts
        uses: actions/download-artifact@<sha> # v4
        with:
          path: artifacts
          pattern: signed-artifacts-*
          merge-multiple: true
```

**Step 3: The `latest.json` generation step needs no changes**

Verify the step's `find` commands still work — they look for `*.app.tar.gz`, `*.app.tar.gz.sig`, `*-setup.exe`, `*-setup.exe.sig` recursively under `artifacts/`. The signed artifacts have the same shape; no changes.

**Step 4: The inline `gh release create` step (from PR-1 Task 1.6) needs no changes**

Verify the file glob still matches the new layout.

**Step 5: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

---

### Task 2.10: Local verification

**Step 1: Lint and typecheck**

```bash
pnpm lint && pnpm typecheck
```

Expected: both pass (the YAML change shouldn't affect either, but verify).

**Step 2: Cargo check on the smoke target**

```bash
cargo build -p snk-smoke-target --release
```

Expected: clean build.

**Step 3: Optionally run actionlint**

```bash
which actionlint && actionlint .github/workflows/*.yml || echo "actionlint not installed (optional)"
```

---

### Task 2.11: Commit the release.yml restructure

**Step 1: Stage**

```bash
git add .github/workflows/release.yml
git status --short
```

Verify only `release.yml` is staged.

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
ci(release): split build/sign + add verify-pubkey + smoke vehicle (#29 #28 #75)

Restructures release.yml from "one matrix job with all secrets co-located
with cargo build" to:

  verify-pubkey → build (matrix, NO secrets) → artifact-verify
                  → environment gate (production-release)
                  → sign-{mac-arm,mac-x64,win-x64} (matrix, scoped secrets)
                  → publish-release

Key points:
- build job runs `tauri build --bundles app,updater|nsis --config '<overlay>'`
  to produce unsigned bundles. NO TAURI_SIGNING_PRIVATE_KEY, NO APPLE_*, NO
  AZURE_* in env during cargo build.
- artifact-verify posts a {platform, file, size, sha256} table to the job
  summary; the reviewer sees it on the Actions UI before approving the gate.
- sign-* jobs each get only the secrets their platform needs. macOS sign
  jobs rebuild .dmg from the signed .app (the unsigned .dmg references the
  unsigned bytes); Windows sign job uses dotnet `sign` CLI then signtool
  verify; both end with `minisign -Sm` on the platform-signed payload.
- verify-pubkey runs first as a cheap fail-fast; smoke-sign-roundtrip.sh
  runs again inside each sign job as defense-in-depth.

Requires: `production-release` environment created in repo Settings with
required reviewer = ehartye and deployment-branches rule Tags v*.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2.12: Push branch and open PR

**Step 1: Push**

```bash
git push -u origin ci/build-sign-split
```

**Step 2: Open PR**

```bash
gh pr create --title "ci(release): split build/sign + verify-pubkey + smoke vehicle (#29, #28, #75)" --body "$(cat <<'EOF'
## Summary

Restructures `release.yml` so signing secrets never enter the `cargo build` environment. Implements #29 (build/sign split), #28 (pubkey-drift CI gate), and #75 (vendored smoke target replacing the cmd.exe roundtrip).

### Job graph

```
tag (v*) → verify-pubkey → build (matrix, no secrets) → artifact-verify
       → environment gate (production-release, ehartye approves)
       → sign-mac-arm / sign-mac-x64 / sign-win-x64 (each gets only its platform's secrets)
       → publish-release (inline `gh release create`)
```

### Key changes

- New `verify-pubkey` job runs first, sign-canary roundtrip with `TAURI_SIGNING_PRIVATE_KEY` against `tauri.conf.json:plugins.updater.pubkey`. Fails fast if they have drifted.
- `build` matrix has NO signing secrets in env — Tauri's bundler is configured via `--config` overlay to skip Authenticode + minisign, and macOS bundler skips codesign without `APPLE_SIGNING_IDENTITY` env.
- `artifact-verify` posts a sha256 table to the job summary; reviewer reads it before approving the environment gate.
- Per-platform sign jobs run the smoke roundtrip against `snk-smoke-target` before touching real artifacts. macOS sign jobs rebuild `.dmg` from the signed `.app` (the unsigned `.dmg` references unsigned bytes).
- `publish-release` consumes `signed-artifacts-*`; everything downstream is unchanged from PR-1.

### Prerequisites

`production-release` environment must exist in repo Settings → Environments:
- Required reviewers: `ehartye`
- Deployment branches and tags: Selected → Tags → `v*`
- No environment secrets (uses repo-level secrets)

If the environment is missing, the sign jobs hang indefinitely.

## Test plan

- [ ] CI passes (`lint-typecheck`, `ts-test`, `rust-test`, `coverage`).
- [ ] Push throwaway tag `v0.0.0-split-test-1`. Watch the workflow:
  - `verify-pubkey` passes
  - `build` matrix passes (all 3 platforms emit unsigned artifacts)
  - `artifact-verify` summary table is populated and looks correct
  - Environment gate prompts for approval → **REJECT** to confirm no secrets are consumed
- [ ] Delete throwaway tag + release.
- [ ] Push throwaway tag `v0.0.0-split-test-2`. Same flow, **APPROVE** the gate:
  - All 3 sign jobs pass (smoke + real sign + verify + minisign)
  - `publish-release` creates the release with all 5 expected assets
  - Verify signed artifacts:
    - `signtool verify /pa /v <installer.exe>` shows valid cert chain back to HartyeTech
    - `codesign --verify --deep --strict --verbose=2 Snapper Keeper.app` reports valid
    - `minisign -V -p <embedded pubkey> -m <installer.exe> -x <installer.exe.sig>` reports verified
- [ ] Delete throwaway tag + release.

Implements #29, #28, #75. Source spec: [`docs/superpowers/specs/2026-05-24-release-pipeline-hardening-design.md`](../blob/main/docs/superpowers/specs/2026-05-24-release-pipeline-hardening-design.md).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

### Task 2.13: Validate via "reject the gate" throwaway tag

**Step 1: Push throwaway tag**

```bash
git tag v0.0.0-split-test-1
git push origin v0.0.0-split-test-1
```

**Step 2: Watch the workflow**

```bash
gh run watch $(gh run list --workflow=release.yml --branch=ci/build-sign-split --limit=1 --json databaseId --jq '.[0].databaseId')
```

Expected sequence:

1. `verify-pubkey` completes (green).
2. `build` matrix (3 jobs) completes (green); each produces an `unsigned-artifacts-*` artifact.
3. `artifact-verify` runs; the job summary contains the sha256 table.
4. Sign jobs queue with status "Waiting" — environment gate prompt visible.

**Step 3: REJECT the gate**

Visit the workflow run in the GitHub Actions UI. Under "Review deployments" for each sign job: click **Reject**.

Expected: each sign job moves to "Skipped" (gate denied). `publish-release` does not run. No release is created.

**Step 4: Confirm no signed artifacts were created**

```bash
gh release view v0.0.0-split-test-1 2>&1 | grep -i 'release not found' && echo "OK: no release created"
```

Expected: "OK: no release created".

**Step 5: Delete the test tag**

```bash
git push origin :v0.0.0-split-test-1
git tag -d v0.0.0-split-test-1
```

---

### Task 2.14: Validate via "approve the gate" throwaway tag

**Step 1: Push throwaway tag**

```bash
git tag v0.0.0-split-test-2
git push origin v0.0.0-split-test-2
```

**Step 2: Watch and APPROVE the gate**

```bash
gh run watch $(gh run list --workflow=release.yml --branch=ci/build-sign-split --limit=1 --json databaseId --jq '.[0].databaseId')
```

When the sign jobs queue at the gate, visit the workflow run in the Actions UI and click **Approve** for each of `sign-mac-arm`, `sign-mac-x64`, `sign-win-x64` (one approval click covers all jobs in the same environment in the same run).

Expected: each sign job runs to completion. `publish-release` creates `v0.0.0-split-test-2` release with `.dmg`, `.app.tar.gz`, `.app.tar.gz.sig`, `-setup.exe`, `-setup.exe.sig`, and `latest.json` assets.

**Step 3: Verify the signed Windows installer**

Download `Snapper.Keeper_<version>_x64-setup.exe` from the release. On a Windows machine:

```powershell
signtool verify /pa /v <downloaded-installer.exe>
```

Expected: shows the cert chain back to HartyeTech, status "Successfully verified".

**Step 4: Verify the signed macOS app**

Download a `Snapper.Keeper_<arch>.app.tar.gz` from the release. On a Mac:

```bash
mkdir -p /tmp/snk-verify && cd /tmp/snk-verify
tar -xzf ~/Downloads/Snapper.Keeper_aarch64.app.tar.gz
codesign --verify --deep --strict --verbose=2 "Snapper Keeper.app"
```

Expected: `Snapper Keeper.app: valid on disk` and `Snapper Keeper.app: satisfies its Designated Requirement`.

**Step 5: Verify the minisign signatures match the embedded pubkey**

On any platform with `minisign` installed and a clone of this repo at the target tag:

```bash
PUBKEY=$(jq -r .plugins.updater.pubkey app/src-tauri/tauri.conf.json | base64 -d)
echo "$PUBKEY" > /tmp/pub.key

# Verify Windows installer signature
minisign -V -p /tmp/pub.key -m <installer.exe> -x <installer.exe.sig>
# Expected: Signature and comment signature verified

# Verify macOS updater payload signature
minisign -V -p /tmp/pub.key -m <Snapper.Keeper_aarch64.app.tar.gz> -x <Snapper.Keeper_aarch64.app.tar.gz.sig>
# Expected: Signature and comment signature verified
```

**Step 6: Delete the test release and tag**

```bash
gh release delete v0.0.0-split-test-2 --yes --cleanup-tag
git fetch --prune --prune-tags
```

**Step 7: Comment on the PR with the verification evidence**

```bash
gh pr comment <PR-NUMBER> --body "Throwaway-tag verification complete:
- v0.0.0-split-test-1: reject-gate flow confirmed (no release created, no sign jobs ran)
- v0.0.0-split-test-2: approve-gate flow confirmed (release built, signtool/codesign/minisign all verified valid)

Both test tags + releases deleted."
```

**Step 8: Merge**

```bash
gh pr merge --squash --delete-branch
```

Sync local main: `git checkout main && git pull`.

---

## PR-3: Nightly audit workflow + per-release SBOM (#46)

**Branch:** `ci/audit-and-sbom` off latest `main` (after PR-2 merged).

Goal: add `audit.yml` (scheduled nightly) with a per-advisory issue lifecycle script. Extend `release.yml` `publish` job with CycloneDX SBOM generation and upload.

---

### Task 3.1: Look up pinned versions for audit + SBOM tools

**Step 1: Look up `cargo-audit` latest stable**

```bash
cargo search cargo-audit | head -3
```

Record as `CARGO_AUDIT_VERSION`.

**Step 2: Look up `cargo-cyclonedx` latest stable**

```bash
cargo search cargo-cyclonedx | head -3
```

Record as `CARGO_CYCLONEDX_VERSION`.

**Step 3: Look up `@cyclonedx/cyclonedx-npm` latest stable**

```bash
pnpm view @cyclonedx/cyclonedx-npm version
```

Record as `CDX_NPM_VERSION`.

**Step 4: Look up `@cyclonedx/cyclonedx-cli` latest stable**

```bash
pnpm view @cyclonedx/cyclonedx-cli version
```

Record as `CDX_CLI_VERSION`.

Keep these versions for use in subsequent tasks.

---

### Task 3.2: Add `audit:auto` label to repo

**Step 1: Create the label**

```bash
gh label create "audit:auto" --color "FBCA04" --description "Filed automatically by the nightly audit workflow" --force
```

`area:security` already exists from the pre-release-review issue creation; reuse it.

---

### Task 3.3: Write the per-advisory issue-sync script

**Files:**
- Create: `.github/scripts/sync-audit-issues.js`

**Step 1: Create the script**

Write to `.github/scripts/sync-audit-issues.js`:

```javascript
// .github/scripts/sync-audit-issues.js
//
// Per-advisory issue lifecycle for the nightly audit workflow.
// Called from .github/workflows/audit.yml via actions/github-script.
//
// Behavior:
//   For each advisory ID in the report:
//     If no open issue with this advisory ID → create one.
//     If one exists → leave it alone (no spam).
//   For each open issue with `audit:auto` whose advisory ID is NOT in the
//   current report:
//     Comment "Advisory no longer detected" and close.
//
// Required env: ECOSYSTEM (cargo|pnpm), REPORT_PATH (file path to audit JSON).

const fs = require('fs');

const ECOSYSTEM_LABELS = {
  cargo: { idPrefix: 'RUSTSEC', name: 'cargo' },
  pnpm:  { idPrefix: 'GHSA',    name: 'pnpm'  },
};

// Normalize a cargo-audit JSON report into our internal advisory shape.
function normalizeCargo(report) {
  const out = [];
  for (const v of (report.vulnerabilities?.list ?? [])) {
    out.push({
      id:             v.advisory?.id ?? 'UNKNOWN',
      severity:       v.advisory?.cvss ?? 'unknown',
      packageName:    v.package?.name ?? 'unknown',
      packageVersion: v.package?.version ?? 'unknown',
      title:          v.advisory?.title ?? '',
      url:            v.advisory?.url ?? '',
      description:    v.advisory?.description ?? '',
    });
  }
  return out;
}

// Normalize a pnpm-audit JSON report into our internal advisory shape.
// `pnpm audit --json` emits an object keyed by advisory id.
function normalizePnpm(report) {
  const out = [];
  const advisories = report.advisories ?? {};
  for (const [id, a] of Object.entries(advisories)) {
    out.push({
      id:             a.github_advisory_id ?? a.cves?.[0] ?? `pnpm-${id}`,
      severity:       a.severity ?? 'unknown',
      packageName:    a.module_name ?? 'unknown',
      packageVersion: a.vulnerable_versions ?? 'unknown',
      title:          a.title ?? '',
      url:            a.url ?? '',
      description:    a.overview ?? '',
    });
  }
  return out;
}

async function main({ github, context, core }) {
  const ecosystem = process.env.ECOSYSTEM;
  const reportPath = process.env.REPORT_PATH;
  if (!ecosystem || !reportPath) {
    core.setFailed('ECOSYSTEM and REPORT_PATH env vars are required.');
    return;
  }

  const eco = ECOSYSTEM_LABELS[ecosystem];
  if (!eco) {
    core.setFailed(`Unknown ECOSYSTEM: ${ecosystem}`);
    return;
  }

  let raw;
  try {
    raw = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  } catch (e) {
    core.warning(`Audit report unreadable at ${reportPath}: ${e.message}`);
    core.info('Treating as empty report (no advisories detected).');
    raw = {};
  }

  const advisories =
    ecosystem === 'cargo' ? normalizeCargo(raw)
    : ecosystem === 'pnpm' ? normalizePnpm(raw)
    : [];

  core.info(`Normalized ${advisories.length} advisories for ${ecosystem}.`);

  // List currently open audit:auto issues for this ecosystem.
  const openIssues = await github.paginate(github.rest.issues.listForRepo, {
    owner: context.repo.owner,
    repo:  context.repo.repo,
    state: 'open',
    labels: 'audit:auto',
    per_page: 100,
  });

  const ecosystemTagInTitle = `[audit/${eco.name}]`;
  const openByAdvisoryId = new Map();
  for (const issue of openIssues) {
    if (!issue.title.startsWith(ecosystemTagInTitle)) continue;
    const m = issue.title.match(/\b(RUSTSEC-\d{4}-\d+|GHSA-[a-z0-9-]+|pnpm-\d+)\b/);
    if (m) openByAdvisoryId.set(m[1], issue);
  }

  const currentIds = new Set(advisories.map(a => a.id));

  // 1) Open new issues for advisories that don't have one.
  for (const adv of advisories) {
    if (openByAdvisoryId.has(adv.id)) {
      core.info(`Already-open issue for ${adv.id}; skipping.`);
      continue;
    }
    const title = `${ecosystemTagInTitle} ${adv.id}: ${adv.packageName}@${adv.packageVersion} — ${adv.title}`;
    const body = [
      `**Ecosystem:** ${eco.name}`,
      `**Advisory:** \`${adv.id}\``,
      `**Severity:** ${adv.severity}`,
      `**Package:** \`${adv.packageName}\` @ \`${adv.packageVersion}\``,
      adv.url ? `**More info:** ${adv.url}` : '',
      '',
      '### Description',
      '',
      adv.description || '_(no description in advisory feed)_',
      '',
      '---',
      '',
      '_Filed automatically by `.github/workflows/audit.yml`. This issue will close itself when the advisory is no longer detected._',
    ].filter(Boolean).join('\n');

    const created = await github.rest.issues.create({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      title,
      body,
      labels: ['area:security', 'audit:auto', 'severity:medium'],
    });
    core.info(`Opened issue #${created.data.number}: ${title}`);
  }

  // 2) Close issues whose advisory is no longer in the report.
  for (const [advId, issue] of openByAdvisoryId.entries()) {
    if (currentIds.has(advId)) continue;
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      issue_number: issue.number,
      body: `Advisory no longer detected as of ${new Date().toISOString().slice(0, 10)} — closing.`,
    });
    await github.rest.issues.update({
      owner: context.repo.owner,
      repo:  context.repo.repo,
      issue_number: issue.number,
      state: 'closed',
    });
    core.info(`Closed issue #${issue.number} (${advId} no longer detected)`);
  }
}

module.exports = main;
```

**Step 2: Verify it parses**

```bash
node -e "require('./.github/scripts/sync-audit-issues.js')" && echo OK
```

Expected: "OK" (the module is loadable; we're not invoking the function).

---

### Task 3.4: Write unit tests for the sync script

**Files:**
- Create: `.github/scripts/__tests__/sync-audit-issues.test.js`
- Create: `.github/scripts/__tests__/fixtures/cargo-empty.json`
- Create: `.github/scripts/__tests__/fixtures/cargo-one-advisory.json`
- Create: `.github/scripts/__tests__/fixtures/pnpm-empty.json`
- Create: `.github/scripts/__tests__/fixtures/pnpm-one-advisory.json`

**Step 1: Create fixtures — cargo empty report**

Write to `.github/scripts/__tests__/fixtures/cargo-empty.json`:

```json
{
  "vulnerabilities": { "found": false, "count": 0, "list": [] }
}
```

**Step 2: Create fixtures — cargo one-advisory report**

Write to `.github/scripts/__tests__/fixtures/cargo-one-advisory.json`:

```json
{
  "vulnerabilities": {
    "found": true,
    "count": 1,
    "list": [
      {
        "advisory": {
          "id": "RUSTSEC-2026-0099",
          "title": "Buffer overflow in fake-crate",
          "description": "A buffer-overflow vuln in fake-crate before 0.5.1.",
          "url": "https://rustsec.org/advisories/RUSTSEC-2026-0099.html",
          "cvss": "high"
        },
        "package": { "name": "fake-crate", "version": "0.5.0" }
      }
    ]
  }
}
```

**Step 3: Create fixtures — pnpm empty report**

Write to `.github/scripts/__tests__/fixtures/pnpm-empty.json`:

```json
{ "advisories": {} }
```

**Step 4: Create fixtures — pnpm one-advisory report**

Write to `.github/scripts/__tests__/fixtures/pnpm-one-advisory.json`:

```json
{
  "advisories": {
    "9999": {
      "github_advisory_id": "GHSA-fake-fake-fake",
      "title": "Prototype pollution in fake-pkg",
      "overview": "fake-pkg before 2.0.0 is vulnerable to prototype pollution.",
      "url": "https://github.com/advisories/GHSA-fake-fake-fake",
      "severity": "high",
      "module_name": "fake-pkg",
      "vulnerable_versions": "<2.0.0"
    }
  }
}
```

**Step 5: Create the test harness**

Write to `.github/scripts/__tests__/sync-audit-issues.test.js`:

```javascript
// Unit tests for .github/scripts/sync-audit-issues.js
//
// Pure Node, no dependencies. Run with `node .github/scripts/__tests__/sync-audit-issues.test.js`.
// Exits 0 on pass, non-zero on failure.

const path = require('path');
const assert = require('assert');

const sync = require('../sync-audit-issues.js');

// Minimal mock of the github + context + core objects passed by github-script.
function makeMockGithub({ existingOpen = [] } = {}) {
  const created = [];
  const closed = [];
  const comments = [];
  return {
    api: {
      rest: {
        issues: {
          listForRepo: async () => ({ data: existingOpen }),
          create:      async (params) => { created.push(params); return { data: { number: 1000 + created.length, ...params } }; },
          update:      async (params) => { if (params.state === 'closed') closed.push(params); return { data: params }; },
          createComment: async (params) => { comments.push(params); return { data: params }; },
        },
      },
    },
    paginate: async (_method, _params) => existingOpen,
    created, closed, comments,
  };
}

function makeCore() {
  const messages = [];
  let failed = null;
  return {
    info:    (m) => messages.push(`info: ${m}`),
    warning: (m) => messages.push(`warn: ${m}`),
    setFailed: (m) => { failed = m; },
    messages, getFailed: () => failed,
  };
}

const context = { repo: { owner: 'ehartye', repo: 'snapper-keeper' } };

async function runCase(name, fn) {
  try {
    await fn();
    console.log(`PASS  ${name}`);
  } catch (e) {
    console.error(`FAIL  ${name}\n  ${e.stack || e.message}`);
    process.exitCode = 1;
  }
}

async function main() {
  const fixtures = path.join(__dirname, 'fixtures');

  await runCase('cargo: empty report → no issues opened, no comments', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-empty.json');
    const gh = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(gh.created.length, 0, 'should not create issues');
    assert.strictEqual(gh.comments.length, 0, 'should not comment');
    assert.strictEqual(gh.closed.length, 0, 'should not close');
  });

  await runCase('cargo: one advisory, no existing issues → opens one', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-one-advisory.json');
    const gh = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(gh.created.length, 1);
    assert.match(gh.created[0].title, /RUSTSEC-2026-0099/);
    assert.match(gh.created[0].title, /\[audit\/cargo\]/);
    assert.deepStrictEqual(
      [...gh.created[0].labels].sort(),
      ['area:security', 'audit:auto', 'severity:medium'],
    );
  });

  await runCase('cargo: one advisory, already-open issue → no new issue, no spam', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-one-advisory.json');
    const gh = makeMockGithub({
      existingOpen: [{
        number: 42,
        title: '[audit/cargo] RUSTSEC-2026-0099: fake-crate@0.5.0 — Buffer overflow',
        labels: [{ name: 'audit:auto' }],
      }],
    });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(gh.created.length, 0);
    assert.strictEqual(gh.comments.length, 0);
    assert.strictEqual(gh.closed.length, 0);
  });

  await runCase('cargo: open advisory issue but advisory no longer in report → close + comment', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = path.join(fixtures, 'cargo-empty.json');
    const gh = makeMockGithub({
      existingOpen: [{
        number: 99,
        title: '[audit/cargo] RUSTSEC-2026-0001: old-crate@1.0.0 — Old vuln',
        labels: [{ name: 'audit:auto' }],
      }],
    });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(gh.created.length, 0);
    assert.strictEqual(gh.comments.length, 1);
    assert.strictEqual(gh.comments[0].issue_number, 99);
    assert.match(gh.comments[0].body, /no longer detected/);
    assert.strictEqual(gh.closed.length, 1);
    assert.strictEqual(gh.closed[0].issue_number, 99);
  });

  await runCase('pnpm: one advisory, no existing → opens one with pnpm tag', async () => {
    process.env.ECOSYSTEM = 'pnpm';
    process.env.REPORT_PATH = path.join(fixtures, 'pnpm-one-advisory.json');
    const gh = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(gh.created.length, 1);
    assert.match(gh.created[0].title, /\[audit\/pnpm\]/);
    assert.match(gh.created[0].title, /GHSA-fake-fake-fake/);
  });

  await runCase('unreadable report file → treats as empty, no failure', async () => {
    process.env.ECOSYSTEM = 'cargo';
    process.env.REPORT_PATH = '/tmp/this-file-does-not-exist-xyz';
    const gh = makeMockGithub({ existingOpen: [] });
    const core = makeCore();
    await sync({ github: gh.api, context, core });
    assert.strictEqual(core.getFailed(), null);
    assert.strictEqual(gh.created.length, 0);
  });
}

main().catch((e) => { console.error(e); process.exitCode = 1; });
```

**Step 6: Run the tests**

```bash
node .github/scripts/__tests__/sync-audit-issues.test.js
```

Expected: six `PASS` lines, no `FAIL`, exit 0.

If any test fails, fix the script in `sync-audit-issues.js` (or the test) until all six pass.

---

### Task 3.5: Commit the audit script + tests

**Step 1: Stage**

```bash
git add .github/scripts/sync-audit-issues.js \
        .github/scripts/__tests__/sync-audit-issues.test.js \
        .github/scripts/__tests__/fixtures/
git status --short
```

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore(ci): add audit issue-sync script + unit tests (precursor to #46)

Per-advisory lifecycle:
- New advisory in report + no open issue → opens one with labels
  area:security, audit:auto, severity:medium
- Already-open issue for the same advisory → no-op (no spam)
- Open audit:auto issue whose advisory is no longer in the report →
  posts a "no longer detected" comment + closes

Normalizes cargo-audit and pnpm-audit JSON into a common advisory shape.
Six unit tests cover empty/new/already-open/closed/cross-ecosystem cases.

No workflow yet; that's the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3.6: Add `audit.yml` workflow

**Files:**
- Create: `.github/workflows/audit.yml`

**Step 1: Write the workflow**

Write to `.github/workflows/audit.yml` (substitute SHAs from PR-1 lookups and the versions from Task 3.1):

```yaml
name: Audit
on:
  schedule:
    - cron: '17 6 * * *'  # 06:17 UTC daily
  workflow_dispatch:

permissions:
  contents: read
  issues: write

jobs:
  cargo-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha> # v4
      - uses: dtolnay/rust-toolchain@<sha> # stable
      - name: Install cargo-audit
        run: cargo install cargo-audit --locked --version <CARGO_AUDIT_VERSION>
      - name: Run cargo audit
        id: audit
        # Write the JSON report regardless of exit code so the sync script
        # can normalize it. Capturing exit avoids the workflow failing on
        # advisories (we surface them via the issue tracker instead).
        run: |
          cargo audit --json > /tmp/audit.json || true
        continue-on-error: true
      - uses: actions/github-script@<sha-for-github-script-v7> # v7
        env:
          ECOSYSTEM: cargo
          REPORT_PATH: /tmp/audit.json
        with:
          script: |
            const sync = require(`${process.env.GITHUB_WORKSPACE}/.github/scripts/sync-audit-issues.js`);
            await sync({ github, context, core });

  pnpm-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha> # v4
      - uses: pnpm/action-setup@<sha> # v3
        with:
          version: 9
      - uses: actions/setup-node@<sha> # v4
        with:
          node-version: 20
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - name: Run pnpm audit
        run: |
          pnpm audit --json > /tmp/audit.json || true
        continue-on-error: true
      - uses: actions/github-script@<sha-for-github-script-v7> # v7
        env:
          ECOSYSTEM: pnpm
          REPORT_PATH: /tmp/audit.json
        with:
          script: |
            const sync = require(`${process.env.GITHUB_WORKSPACE}/.github/scripts/sync-audit-issues.js`);
            await sync({ github, context, core });
```

Resolve the SHA for `actions/github-script@v7` using the same lookup pattern as PR-1 Task 1.1 if it wasn't part of the original sweep.

**Step 2: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/audit.yml'))"
```

---

### Task 3.7: Validate the audit workflow via `workflow_dispatch`

**Step 1: Push the branch with audit.yml**

```bash
git add .github/workflows/audit.yml
git commit -m "ci(audit): add nightly cargo-audit + pnpm-audit workflow (#46)"
git push -u origin ci/audit-and-sbom
```

**Step 2: Trigger via workflow_dispatch**

```bash
gh workflow run audit.yml --ref ci/audit-and-sbom
```

**Step 3: Watch the run**

```bash
gh run watch $(gh run list --workflow=audit.yml --branch=ci/audit-and-sbom --limit=1 --json databaseId --jq '.[0].databaseId')
```

Expected (assuming no current advisories): both jobs complete green, no new issues opened.

If there ARE current advisories in either ecosystem: the script opens issues with the right labels. Verify with:

```bash
gh issue list --label "audit:auto" --state open
```

**Step 4: Verify the "already open" case by re-running**

```bash
gh workflow run audit.yml --ref ci/audit-and-sbom
gh run watch $(gh run list --workflow=audit.yml --branch=ci/audit-and-sbom --limit=1 --json databaseId --jq '.[0].databaseId')
```

Expected: no duplicate issues created. Log line should say "Already-open issue for <ID>; skipping" for any advisory.

**Step 5: If the audit opened test issues that aren't real advisories, close them by hand**

(Shouldn't happen if the repo's current state genuinely has no advisories.)

```bash
gh issue list --label "audit:auto" --state open --json number --jq '.[].number' | xargs -I{} gh issue close {} --comment "Test cleanup from audit workflow validation"
```

---

### Task 3.8: Add SBOM generation to the `publish-release` job

**Files:**
- Modify: `.github/workflows/release.yml` (the `publish-release` job)

**Step 1: Add SBOM steps after `gh release create`**

In `release.yml`'s `publish-release` job, after the `Create GitHub Release` step, add:

```yaml
      - uses: dtolnay/rust-toolchain@<sha> # stable

      - name: Install SBOM tools
        run: |
          cargo install cargo-cyclonedx --locked --version <CARGO_CYCLONEDX_VERSION>

      - name: Generate Rust SBOM
        run: |
          # cargo cyclonedx emits ./bom.cdx.json (or per-crate files with
          # --output-pattern).
          cargo cyclonedx --format json --output-pattern bom -- --workspace

      - name: Generate npm SBOM
        run: |
          pnpm dlx @cyclonedx/cyclonedx-npm@<CDX_NPM_VERSION> \
            --output-format JSON --output-file ./bom-npm.cdx.json

      - name: Merge SBOMs
        run: |
          pnpm dlx @cyclonedx/cyclonedx-cli@<CDX_CLI_VERSION> merge \
            --input-files ./bom.cdx.json ./bom-npm.cdx.json \
            --output-file ./sbom.cdx.json

      - name: Validate SBOM
        run: |
          pnpm dlx @cyclonedx/cyclonedx-cli@<CDX_CLI_VERSION> validate \
            --input-file ./sbom.cdx.json

      - name: Attach SBOM to release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: gh release upload "$GITHUB_REF_NAME" ./sbom.cdx.json --clobber
```

Substitute the SHAs (from PR-1) and the versions (from Task 3.1).

**Step 2: YAML sanity check**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

---

### Task 3.9: Commit SBOM extension + push

**Step 1: Stage**

```bash
git add .github/workflows/release.yml
git status --short
```

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
ci(release): generate + attach CycloneDX SBOM per release (#46)

Adds Rust (cargo-cyclonedx) and npm (@cyclonedx/cyclonedx-npm) SBOM
generation to the publish-release job, merges into a single
sbom.cdx.json via @cyclonedx/cyclonedx-cli, validates, and attaches as
a release asset alongside the platform installers.

Completes #46 alongside the nightly audit workflow in the prior commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Step 3: Push**

```bash
git push
```

---

### Task 3.10: Open PR

**Step 1:**

```bash
gh pr create --title "ci(audit+release): nightly cargo/pnpm audit + per-release CycloneDX SBOM (#46)" --body "$(cat <<'EOF'
## Summary

Implements #46.

### Nightly audit (`audit.yml`)

- Scheduled daily at 06:17 UTC; also `workflow_dispatch`.
- `cargo-audit` and `pnpm-audit` each run in their own job and emit JSON.
- Both feed `.github/scripts/sync-audit-issues.js`, which:
  - Opens a new issue for any advisory ID not already represented (labels: `area:security`, `audit:auto`, `severity:medium`).
  - Leaves already-open issues alone (no comment spam).
  - Comments + closes any `audit:auto` issue whose advisory ID is no longer in the report.
- Six unit tests in `.github/scripts/__tests__/sync-audit-issues.test.js` cover empty / new-advisory / already-open / resolved / cross-ecosystem / unreadable-report cases.

### Per-release SBOM (`release.yml`)

- After `gh release create`, the `publish-release` job runs `cargo cyclonedx` + `@cyclonedx/cyclonedx-npm`, merges via `@cyclonedx/cyclonedx-cli`, validates, and uploads `sbom.cdx.json` as a release asset.

## Test plan

- [x] Unit tests pass locally: `node .github/scripts/__tests__/sync-audit-issues.test.js` → 6 PASS
- [x] `workflow_dispatch` runs `audit.yml` cleanly on the branch (validates the wiring; current repo state has no live advisories, so no issues opened — log line "Normalized 0 advisories" confirms)
- [x] Re-run `audit.yml` → no duplicates (already-open path)
- [ ] Push throwaway tag `v0.0.0-sbom-test-1` after merge, confirm `sbom.cdx.json` is attached to the release, valid CycloneDX

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

### Task 3.11: Validate SBOM via throwaway release tag

**Step 1: Push throwaway tag**

```bash
git tag v0.0.0-sbom-test-1
git push origin v0.0.0-sbom-test-1
```

**Step 2: Watch the workflow**

```bash
gh run watch $(gh run list --workflow=release.yml --branch=ci/audit-and-sbom --limit=1 --json databaseId --jq '.[0].databaseId')
```

The full release pipeline runs — including the environment gate (approve it).

**Step 3: Verify SBOM is attached**

```bash
gh release view v0.0.0-sbom-test-1 --json assets --jq '.assets[].name'
```

Expected: `sbom.cdx.json` is in the list alongside the installers.

**Step 4: Download and validate the SBOM locally**

```bash
gh release download v0.0.0-sbom-test-1 --pattern 'sbom.cdx.json' --dir /tmp/
pnpm dlx @cyclonedx/cyclonedx-cli@<CDX_CLI_VERSION> validate --input-file /tmp/sbom.cdx.json
```

Expected: "BOM validated successfully" (or equivalent success message).

**Step 5: Spot-check the SBOM content**

```bash
jq '.components | length' /tmp/sbom.cdx.json
```

Expected: a non-zero count (should be a few hundred — Rust crates + npm deps).

```bash
jq '.components[].name' /tmp/sbom.cdx.json | grep -E '(tauri|react|tokio)' | head
```

Expected: some matches; confirms the SBOM captures both ecosystems.

**Step 6: Delete the test release and tag**

```bash
gh release delete v0.0.0-sbom-test-1 --yes --cleanup-tag
git fetch --prune --prune-tags
```

**Step 7: Comment on PR + merge**

```bash
gh pr comment <PR-NUMBER> --body "Throwaway-tag SBOM validation complete: sbom.cdx.json attached to v0.0.0-sbom-test-1 release, validates via cyclonedx-cli, captures both Rust and npm components. Tag + release deleted."
gh pr merge --squash --delete-branch
```

Sync local main: `git checkout main && git pull`.

---

## Done definition

All three PRs merged. Final state:

- `release.yml`: verify-pubkey → build (matrix, no secrets) → artifact-verify → environment gate → sign-* (matrix, scoped secrets) → publish-release (with SBOM).
- `audit.yml`: scheduled nightly per-advisory issue lifecycle for cargo + pnpm.
- `ci.yml` + `release.yml`: every `uses:` SHA-pinned; Tesseract chocolatey + dotnet sign version-pinned.
- New scripts: `scripts/verify-pubkey.sh`, `scripts/smoke-sign-roundtrip.sh`, `.github/scripts/sync-audit-issues.js` + tests.
- New crate: `tests/fixtures/smoke-target/`.

Issues #28, #29, #30, #46, #75 closed.

## Skill handoff

After all three PRs merge, the cluster is complete. No further skill invocations required; the next pre-release work picks up from a fresh skill chain.
