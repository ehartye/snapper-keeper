# Release Pipeline Hardening — Design

Date: 2026-05-24
Author: Eric Hartye + Claude (Opus 4.7)
Status: Draft (awaiting Eric's review)

Covers issues:

- [#28 — Re-derive pubkey from `TAURI_SIGNING_PRIVATE_KEY` and diff against `tauri.conf.json` before build (HIGH)](https://github.com/ehartye/snapper-keeper/issues/28)
- [#29 — Split build job (no secrets) from sign+publish job (HIGH)](https://github.com/ehartye/snapper-keeper/issues/29)
- [#30 — Pin CI actions by SHA + pin/verify Tesseract chocolatey source (HIGH)](https://github.com/ehartye/snapper-keeper/issues/30)
- [#46 — Nightly `cargo audit` + `pnpm audit` + per-release SBOM (MED)](https://github.com/ehartye/snapper-keeper/issues/46)
- [#75 — Replace cmd.exe smoke with vendored test artifact + verify roundtrip (LOW)](https://github.com/ehartye/snapper-keeper/issues/75)

---

## 1. Goal

Bring `release.yml` from "single matrix job with all secrets co-located with cargo build" to a job graph where signing secrets never enter the cargo build environment, every third-party action is pinned by commit SHA, a human approval gate sits between build and sign, and supply-chain scanning is wired up (nightly advisories + per-release SBOM).

The release pipeline has been exercised many times in the `v0.0.0-testN` → `v0.1.0` cycle. The shape of `build → publish` is proven; this design re-shapes the internals without changing the externally observable contract (a tagged release ends up at `github.com/ehartye/snapper-keeper/releases/v*` with signed `.exe`, `.app.tar.gz`, `.dmg`, and `latest.json` for the updater).

## 2. Threat model

Concrete: any transitive `build.rs`, proc-macro, or linker hook executes during `cargo build` and reads process env. Snapper-keeper pulls ~400+ transitive crates. A single compromised dep can exfiltrate every env var, including:

- **`TAURI_SIGNING_PRIVATE_KEY`** (Ed25519 minisign) — the updater key. Compromise = attacker forges `latest.json` and updater payloads, signed-as-us. The pubkey is baked into every installed binary; **no revocation path**. Worst-case blast radius.
- **`AZURE_CLIENT_SECRET`** (Trusted Signing SP) — Authenticode-sign arbitrary binaries as HartyeTech. Recoverable by rotating the SP and rebuilding cert reputation.
- **`APPLE_CERTIFICATE`** + `APPLE_PASSWORD` — codesign + notarize arbitrary binaries as Developer ID. Recoverable by contacting Apple to revoke.

This design moves all three secret classes out of the `cargo build` environment.

## 3. Cluster boundary and PR sequencing

Single design covering all 5 issues; three PRs at implementation time:

| PR | Issues | Scope |
|---|---|---|
| **PR-1** | #30 | Mechanical SHA pinning across `ci.yml` + `release.yml`. Drop `--prerelease` from `dotnet sign` install. Pin Tesseract chocolatey + SHA256-verify. Replace `softprops/action-gh-release` with inline `gh release create`. Independent; lands first. |
| **PR-2** | #29 + #28 + #75 | Coupled. #29 defines the sign job that #28 (pubkey diff) and #75 (smoke vehicle) inhabit. Largest restructure. Includes the new `production-release` environment + artifact-verification gate. |
| **PR-3** | #46 | New `audit.yml` (scheduled) + SBOM generation in `release.yml`. Additive; lands last. |

Two specs were considered (signing vs supply-chain) and rejected: the SBOM step lives inside `release.yml` alongside the signing work; splitting the narrative splits a single coherent change. Single spec, three PRs.

## 4. Job graph (post-redesign)

```
                       tag pushed (v*)
                              │
                              ▼
              ┌───────────────────────────────┐
              │ verify-pubkey  (Ubuntu)       │  #28 — fails fast if minisign
              │  - TAURI_SIGNING_PRIVATE_KEY  │       priv-key drifts from
              │  - reads tauri.conf.json      │       embedded pubkey
              │  - NO other secrets           │
              └───────────────┬───────────────┘
                              │ pass
                              ▼
              ┌───────────────────────────────┐
              │ build  (matrix: 3 platforms)  │  #29 — cargo build + tauri build
              │  - NO signing secrets in env  │       with --config overlay
              │  - --config disables signing  │       emitting unsigned .exe /
              │  - uploads UNSIGNED artifacts │       unsigned .app.tar.gz /
              │    + sha256 manifest          │       unsigned .dmg
              └───────────────┬───────────────┘
                              │ all platforms green
                              ▼
              ┌───────────────────────────────┐
              │ artifact-verify  (Ubuntu)     │  posts size + sha256 table to
              │  - no secrets                 │  job summary; the table is what
              │  - downloads all artifacts    │  the reviewer sees before
              │  - recomputes sha256          │  approving
              │  - prints summary table       │
              └───────────────┬───────────────┘
                              │
                              ▼
                  ╔═══════════════════════╗
                  ║ environment:          ║  human click in Actions UI;
                  ║   production-release  ║  one approval grants all
                  ║   (required reviewer) ║  downstream sign-* jobs
                  ╚═══════════╤═══════════╝
                              │ approved
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
   │ sign-mac-arm │  │ sign-mac-x64 │  │ sign-win-x64 │  #29 — each gets ONLY
   │ macos-latest │  │ macos-15-    │  │ windows-     │       the secrets its
   │              │  │ intel        │  │ latest       │       platform needs
   │ APPLE_*      │  │ APPLE_*      │  │ AZURE_*      │  #75 — each runs the
   │ TAURI_SIGN_* │  │ TAURI_SIGN_* │  │ TAURI_SIGN_* │       smoke vehicle as
   │              │  │              │  │              │       first step
   │ smoke roundtrip │ smoke roundtrip │ smoke roundtrip
   │ codesign     │  │ codesign     │  │ sign CLI     │
   │  + redmg →   │  │  + redmg →   │  │  (Azure) →   │
   │ notarize →   │  │ notarize →   │  │ signtool     │
   │ minisign →   │  │ minisign →   │  │  verify →    │
   │ upload signed│  │ upload signed│  │ minisign →   │
   └──────┬───────┘  └──────┬───────┘  │ upload signed│
          │                 │          └──────┬───────┘
   └──────┬───────┘  └──────┬───────┘         │
          │                 │                  │
          └─────────────────┼──────────────────┘
                            ▼
              ┌───────────────────────────────┐
              │ publish  (Ubuntu)             │  downloads signed artifacts,
              │  - no signing secrets         │  generates latest.json,
              │  - generates latest.json      │  creates GH release via gh CLI
              │  - inline `gh release create` │  (not third-party action)
              │  - generates + attaches SBOM  │  #46 — SBOM lives here
              │  - permissions: contents:write│
              └───────────────────────────────┘
```

## 5. Per-job permissions

| Job | `contents` | `id-token` | Secrets in env |
|---|---|---|---|
| `verify-pubkey` | `read` | none | `TAURI_SIGNING_PRIVATE_KEY` + password |
| `build` (matrix) | `read` | none | **none** |
| `artifact-verify` | `read` | none | none |
| `sign-mac-arm` / `sign-mac-x64` | `read` | none | `APPLE_*` + `TAURI_SIGNING_PRIVATE_KEY` + password |
| `sign-win-x64` | `read` | none | `AZURE_*` + `TAURI_SIGNING_PRIVATE_KEY` + password |
| `publish` | `write` | none | none (uses `GITHUB_TOKEN`) |

Least-privilege: even if the `build` job is compromised, it can't push commits or read signing secrets. Even if a sign job is compromised, it can only sign on the platform it owns and can't create the release.

## 6. Key technical decisions

### 6.1 Disabling signing during build

`tauri build --target X --config '{"bundle":{"windows":{"signCommand":""}},"plugins":{"updater":{"pubkey":""}}}'`. Empty `signCommand` causes the Windows bundler to skip Authenticode. Absent `APPLE_SIGNING_IDENTITY` env causes the macOS bundler to skip `codesign`. Empty `pubkey` (combined with absent `TAURI_SIGNING_PRIVATE_KEY` env) causes the updater bundler to skip minisign. Output: bundles identical to today's signed output minus the signing — same paths, same filenames, no `.sig` files.

### 6.2 Post-bundle signing in sign jobs

Each sign job:

1. Downloads the unsigned bundle from the `unsigned-artifacts-<label>` artifact.
2. Runs `scripts/smoke-sign-roundtrip.sh` (Section 7.4) — proves the signing toolchain works against a throwaway stub binary before touching real artifacts. The smoke script itself starts with a `scripts/verify-pubkey.sh` call as defense-in-depth (the `verify-pubkey` job already ran upstream, but secrets may rotate between jobs in unusual cases).
3. Signs the platform binary:
   - **Windows**: `sign code artifact-signing -ase https://eus.codesigning.azure.net -asa HartyeTech -ascp snapper-keeper -v Information <installer.exe>` (the dotnet `sign` CLI; calls Azure Trusted Signing). Then `signtool verify /pa /v <installer.exe>` to confirm the cert chain back to HartyeTech before proceeding.
   - **macOS**: `codesign --sign "$APPLE_SIGNING_IDENTITY" --options runtime --timestamp --deep <Snapper Keeper.app>`. Then `codesign --verify --deep --strict --verbose=2 <Snapper Keeper.app>` to confirm valid-on-disk.
4. **macOS only — rebuild `.dmg` from the signed `.app`**: the unsigned `.dmg` produced by the build job contains the unsigned `.app`. Discard it; build a fresh `.dmg` with the signed `.app` inside via `create-dmg` or `hdiutil create`. This step exists because the `.dmg` is a content-addressed bundle of the `.app` — signing the `.app` after `.dmg` creation produces a `.dmg` that still references the unsigned bytes.
5. Notarizes (macOS only): `xcrun notarytool submit <rebuilt.dmg> --apple-id ... --wait` + `xcrun stapler staple <rebuilt.dmg>`. The `.app.tar.gz` does NOT get notarized — Apple only notarizes installer formats (`.dmg`, `.pkg`, `.app`); the tarball is treated as a payload.
6. Re-bundles `.app` into `.app.tar.gz` (macOS) with the now-signed `.app` inside. This is the updater payload.
7. Runs `minisign -Sm` on the final signed binary (the `.app.tar.gz` on macOS, the `-setup.exe` on Windows) to produce the `.sig` updater file. **This step must be last** — the minisign signature is over the bytes of the platform-signed binary. The `.dmg` is NOT minisigned (per current behavior; the updater only consumes `.app.tar.gz`).
8. Renames per-arch on macOS (per `MEMORY.md → reference_tauri2_updater_artifacts.md` — Tauri 2 emits `Snapper Keeper.app.tar.gz` with no arch suffix; both arm64 and x64 collide on the same release asset name).
9. Uploads `signed-artifacts-<label>` for the `publish` job.

### 6.3 Apple keychain handling

Cert import + keychain unlock moves from build job into each `sign-mac-*` job (only those need codesign). The 2-hour idle timeout (per `MEMORY.md → reference_macos_keychain_timeout_ci.md`) carries over.

### 6.4 Artifact-verify summary

After the build matrix completes, the `artifact-verify` job runs on Ubuntu. It:

1. Downloads all `unsigned-artifacts-*`.
2. Walks each artifact directory; for every file emits `{platform, filename, size_bytes, sha256}`.
3. Writes a markdown table to `$GITHUB_STEP_SUMMARY`:

```markdown
| Platform | File | Size | SHA-256 |
|---|---:|---:|---|
| macOS-arm64 | Snapper Keeper.app.tar.gz | 12,345,678 | abc123... |
| macOS-arm64 | Snapper Keeper.dmg | 23,456,789 | def456... |
| macOS-x64 | Snapper Keeper.app.tar.gz | 12,398,765 | 789abc... |
| Windows-x64 | snapper-keeper_0.X.Y_x64-setup.exe | 14,567,890 | 0123ab... |
```

The reviewer sees this table on the Actions UI before clicking approve. Wrong size or missing file = reject before signing secrets are consumed.

### 6.5 Environment gate

`production-release` environment created in repo Settings → Environments. Configuration:

- **Required reviewers**: `ehartye`
- **Deployment branches and tags**: `Selected branches and tags` → add rule `Tags` `v*`
- **Wait timer**: 0
- **Environment secrets**: none (secrets stay at repo level; environment provides the gate, not secret scoping in this design)

One approval grants all jobs referencing the environment in the same workflow run.

If the user rejects: pipeline halts. No signing secrets consumed. No artifacts uploaded to a release.

### 6.6 Verify-pubkey via sign-canary roundtrip

Tauri 2 has no `tauri signer sign --print-pub-key` CLI flag (Tauri 1 had a different flag; Tauri 2 dropped it). The robust pattern is to sign a known string with the private key and verify the signature with the embedded pubkey. If verify succeeds, the keys match.

`scripts/verify-pubkey.sh`:

```bash
#!/usr/bin/env bash
# Fail loudly if TAURI_SIGNING_PRIVATE_KEY does not match the pubkey embedded
# in app/src-tauri/tauri.conf.json plugins.updater.pubkey.
#
# Required env: TAURI_SIGNING_PRIVATE_KEY (base64), TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# Side effects: writes/deletes temp files under TMPDIR; no network.
set -euo pipefail

: "${TAURI_SIGNING_PRIVATE_KEY:?TAURI_SIGNING_PRIVATE_KEY must be set}"
: "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:?TAURI_SIGNING_PRIVATE_KEY_PASSWORD must be set}"

WORK=$(mktemp -d)
trap "rm -rf '$WORK'" EXIT

echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$WORK/priv.key"
echo "snapper-keeper-pubkey-drift-canary" > "$WORK/canary.txt"

# Sign the canary with the secret-held private key.
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

`minisign` is installed on the runner via `apt-get install minisign` (Ubuntu), `brew install minisign` (macOS), or `choco install minisign` (Windows). All three have it; the version pinning per platform happens in the workflow YAML.

## 7. Per-issue implementation detail

### 7.1 #28 — verify-pubkey job

New first job in `release.yml`. Runs `scripts/verify-pubkey.sh` (Section 6.6). Single failure mode, single recovery path. The same script is invoked at the top of each sign job for defense-in-depth.

### 7.2 #29 — Build job (no secrets) + sign jobs

Replaces today's `build-and-release` matrix. Build matrix step:

```yaml
- name: Build Tauri app (unsigned)
  # NO env block for signing secrets — none in process env during cargo build.
  run: |
    pnpm tauri build --target ${{ matrix.target }} \
      --config '{"bundle":{"windows":{"signCommand":""}},"plugins":{"updater":{"pubkey":""}}}'

- name: Upload unsigned artifacts
  uses: actions/upload-artifact@<sha> # v4
  with:
    name: unsigned-artifacts-${{ matrix.label }}
    path: |
      target/${{ matrix.target }}/release/bundle/macos/Snapper Keeper.app/
      target/${{ matrix.target }}/release/bundle/nsis/*-setup.exe
    if-no-files-found: error
```

Each sign job downloads the artifact, runs the post-bundle signing sequence from Section 6.2, uploads `signed-artifacts-<label>`.

The macOS sign jobs need the `.app` directory (not the `.app.tar.gz` and not the `.dmg`) so they can codesign the app contents and then construct both the `.dmg` and the `.app.tar.gz` from the signed `.app`. The build job uses Tauri's `--bundles` flag to skip the `.dmg` target (the unsigned .dmg would only be discarded by the sign job anyway). Concretely:

- macOS build invocation: `pnpm tauri build --target <target> --bundles app,updater --config '<overlay>'` — emits only the `.app` (and unsigned `.app.tar.gz` which is also discarded by the sign job in favor of a fresh tar of the signed app).
- Windows build invocation: `pnpm tauri build --target <target> --bundles nsis --config '<overlay>'` — emits only the unsigned `-setup.exe`.

Plan-time validation: confirm `--bundles app,updater` is the correct flag syntax in Tauri 2 (may be `--bundles app --bundles updater` or similar). If the flag is unavailable, fallback is to emit all bundles and discard the unsigned `.dmg` in the sign job.

### 7.3 #30 — SHA pinning sweep

Mechanical. For each `uses: org/action@vX`, resolve to a commit SHA:

```bash
# Resolve a tag → commit SHA (follow annotated-tag indirection if needed).
TAG=v4
SHA=$(gh api repos/actions/checkout/git/refs/tags/$TAG --jq '.object.sha')
TYPE=$(gh api repos/actions/checkout/git/refs/tags/$TAG --jq '.object.type')
if [ "$TYPE" = "tag" ]; then
  SHA=$(gh api repos/actions/checkout/git/tags/$SHA --jq '.object.sha')
fi
echo "uses: actions/checkout@$SHA # $TAG"
```

Run once per action, write SHAs into the YAML with `# <tag>` floating-tag comments. Actions touched:

- `actions/checkout` (used in every job)
- `pnpm/action-setup`
- `actions/setup-node`
- `actions/setup-dotnet`
- `dtolnay/rust-toolchain`
- `Swatinem/rust-cache`
- `actions/upload-artifact`
- `actions/download-artifact`
- `actions/github-script` (added in PR-3)

Also in this sweep:

- `dotnet tool install --global --prerelease sign` → `dotnet tool install --global sign --version <pinned>`. Lookup the current stable version at PR-1 time and pin it.
- Tesseract chocolatey: `choco install tesseract --version=<pinned> --no-progress --confirm --requirechecksums`. Independent SHA256 verify of the downloaded MSI (chocolatey's own checksum is in its manifest; we want our own gate above that):

```powershell
$ExpectedSha = 'KNOWN_SHA256_HEX'  # captured once at PR-1 authoring; committed.
$MsiPath = (Get-ChildItem "$env:ChocolateyInstall\lib\tesseract" `
                          -Filter '*.msi' -Recurse | Select-Object -First 1).FullName
$ActualSha = (Get-FileHash -Algorithm SHA256 $MsiPath).Hash
if ($ActualSha -ne $ExpectedSha) {
  Write-Error "Tesseract MSI SHA256 mismatch: expected $ExpectedSha, got $ActualSha"
  exit 1
}
```

- `softprops/action-gh-release@v2` → inline:

```yaml
- name: Create GitHub Release
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    gh release create "$GITHUB_REF_NAME" \
      --title "$GITHUB_REF_NAME" \
      --generate-notes \
      artifacts/**/*.dmg \
      artifacts/**/*.app.tar.gz \
      artifacts/**/*.app.tar.gz.sig \
      artifacts/**/*-setup.exe \
      artifacts/**/*-setup.exe.sig \
      artifacts/sbom.cdx.json \
      artifacts/latest.json
```

`gh` CLI is preinstalled on `ubuntu-latest`.

### 7.4 #75 — Smoke vehicle (stub binary + scripts)

New crate at `tests/fixtures/smoke-target/`:

```
tests/fixtures/smoke-target/
  Cargo.toml          # publish=false, name=snk-smoke-target, binary
  src/main.rs         # fn main() { println!("snapper-keeper sign smoke ok"); }
  build.rs            # Windows: embed manifest declaring asInvoker (per
                      # MEMORY.md → "Windows UAC installer detection heuristic")
  smoke-target.exe.manifest
```

Crate added to the workspace `Cargo.toml` `[workspace.members]`. The name `snk-smoke-target` deliberately avoids the substrings `update`, `setup`, `install` (per `MEMORY.md` → Windows UAC heuristic).

`scripts/smoke-sign-roundtrip.sh` (POSIX shell with platform branches; called from all sign jobs):

```bash
#!/usr/bin/env bash
# Smoke-test the signing toolchain against a throwaway stub binary BEFORE
# touching real release artifacts. If any step fails, the sign job aborts and
# no real artifact gets signed.
#
# Required env (varies by platform):
#   Windows: AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET
#   macOS: APPLE_SIGNING_IDENTITY, build.keychain unlocked
#   All: TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD
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
    echo "::error::Unknown platform: $PLATFORM"; exit 2
    ;;
esac

echo "Smoke roundtrip passed for $PLATFORM."
```

### 7.5 #46 — Nightly audit + per-release SBOM

**Nightly audit** at `.github/workflows/audit.yml`:

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
      - run: cargo install cargo-audit --locked --version <pinned>
      - id: audit
        run: cargo audit --json > /tmp/audit.json
        continue-on-error: true
      - uses: actions/github-script@<sha> # v7
        env:
          ECOSYSTEM: cargo
          REPORT_PATH: /tmp/audit.json
        with:
          script: |
            const sync = require('./.github/scripts/sync-audit-issues.js');
            await sync({ github, context, core });

  pnpm-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha> # v4
      - uses: pnpm/action-setup@<sha> # v3
      - uses: actions/setup-node@<sha> # v4
      - run: pnpm install --frozen-lockfile
      - run: pnpm audit --json > /tmp/audit.json
        continue-on-error: true
      - uses: actions/github-script@<sha> # v7
        env:
          ECOSYSTEM: pnpm
          REPORT_PATH: /tmp/audit.json
        with:
          script: |
            const sync = require('./.github/scripts/sync-audit-issues.js');
            await sync({ github, context, core });
```

**Per-advisory issue lifecycle** at `.github/scripts/sync-audit-issues.js`:

- Parse the audit JSON (cargo-audit and pnpm-audit have different shapes; the script normalizes to `{id, severity, packageName, packageVersion, title, url, description}` records).
- For each advisory:
  - Search open issues with labels `area:security` + `audit:auto` and title containing the advisory ID.
  - If none open → create. Title: `[audit] <ID>: <package>@<version> — <title>`. Body: ecosystem, severity, affected package + version, advisory URL, full description. Labels: `area:security`, `audit:auto`, `severity:medium` (default; can be overridden manually).
  - If one exists → leave it alone (no update spam; human triages once).
- For each open issue with `audit:auto` label whose advisory ID is NOT in any current report:
  - Post a comment "Advisory no longer detected as of <UTC date> — closing." Close the issue.

Two new labels: `audit:auto` (yellow, `description: "Filed automatically by the nightly audit workflow"`). `area:security` already exists.

**Per-release SBOM** added to the `publish` job in `release.yml`, after `gh release create`:

```yaml
- name: Install SBOM tools
  run: |
    cargo install cargo-cyclonedx --locked --version <pinned>

- name: Generate Rust SBOM
  run: |
    cargo cyclonedx --format json --output-pattern bom \
      --workspace -- --features ""
    # Emits ./bom.cdx.json at workspace root.

- name: Generate npm SBOM
  run: |
    pnpm dlx @cyclonedx/cyclonedx-npm@<pinned> \
      --output-format JSON --output-file ./bom-npm.cdx.json

- name: Merge SBOMs
  run: |
    pnpm dlx @cyclonedx/cyclonedx-cli@<pinned> merge \
      --input-files ./bom.cdx.json ./bom-npm.cdx.json \
      --output-file ./sbom.cdx.json

- name: Attach SBOM to release
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: gh release upload "$GITHUB_REF_NAME" ./sbom.cdx.json --clobber
```

`@cyclonedx/cyclonedx-npm` is the official npm/pnpm-compatible tool from the CycloneDX maintainers. `cargo cyclonedx` is the Rust equivalent. `@cyclonedx/cyclonedx-cli` merges to a single SBOM file.

## 8. Error handling, observability, rollback

### 8.1 Failure modes

| Job | Failure → effect | Recovery |
|---|---|---|
| `verify-pubkey` | Workflow fails before any build. Tag stays; release not cut. | Rotate `TAURI_SIGNING_PRIVATE_KEY` to match embedded pubkey, OR commit a new `tauri.conf.json` pubkey + bump version + re-tag. |
| `build` (any platform) | Workflow fails for that platform; matrix continues for others (`fail-fast: false`). | Fix platform-specific issue; re-tag. |
| `artifact-verify` | Fails if any expected artifact is missing or sha256 cannot be computed. Sign jobs do not start (gate never reached). | Investigate the empty build matrix output. |
| Gate not approved | Pipeline halts (until manual reject or 30-day auto-expiry). | Reject in Actions UI; no signing secrets consumed. |
| `sign-*` smoke | Smoke roundtrip fails before real artifacts are touched. No signed asset reaches `publish`. | Per-platform debug: cert chain expired, Azure SP rotated without updating GitHub secret, keychain timeout, etc. |
| `sign-*` real signing after smoke passed | Unusual. Workflow fails; partial artifacts in storage but `publish` doesn't run. | Re-run the failing sign job from Actions UI (re-uses approved environment). |
| `publish` | Partial release published. | Manually `gh release delete`, remove tag, fix, re-tag. |

### 8.2 Observability additions

1. **Per-job step summaries** (`$GITHUB_STEP_SUMMARY`): `artifact-verify` writes the size+sha256 table the reviewer reads before approving; sign jobs write smoke result + signature verification output; `publish` writes the final asset list + URLs.
2. **Explicit pubkey-drift error**: `verify-pubkey` failure prints "TAURI_SIGNING_PRIVATE_KEY does not match tauri.conf.json plugins.updater.pubkey" with the line number of the embedded pubkey.
3. **Signed-artifact hash table** in the `publish` job summary: `{filename, sha256, bytes}` for every release asset.
4. **No silent secret use**: every step that touches secrets logs the tool name and the file it produced — never the secret value.

### 8.3 Rollback shape

This cluster does not change the post-release rollback story:

- A published release exists; users on auto-update will get it. No kill switch today (`CP1` covers `latest.json` signing + downgrade floor; out of scope here).
- Manual rollback: `gh release delete v0.X.Y` removes the assets and `latest.json`. The auto-updater falls back to "no update available" on next check (it consults `/releases/latest/`). Users already on the bad version stay there; need a new higher-versioned release.

What this cluster *does* add: the **pre-publish** rollback opportunity. The reviewer sees the artifact-verify table at the environment gate; if anything looks wrong (size off, asset missing, sha256 doesn't match what they expect), they reject and no signed bytes hit the wire.

## 9. Testing

Workflows are configuration, not code; testing is "run against a throwaway tag and observe."

### PR-1 (SHA pinning)

- `pnpm lint && pnpm typecheck`; `cargo fmt -- --check`; `cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings`; `cargo test --workspace --exclude snapper-keeper-app`.
- Branch push: `ci.yml` runs as normal CI. Must pass green. Validates every SHA we pinned is real and reachable.
- Throwaway tag `v0.0.0-sha-pin-test`: watch `release.yml` end-to-end. Delete tag + release after.

### PR-2 (build/sign split + pubkey + smoke)

- `cargo build -p snk-smoke-target` locally on at least one platform.
- Branch push: full `ci.yml` runs (no signing; just compile + tests).
- Throwaway tag `v0.0.0-split-test-1`: validates `verify-pubkey` → `build` (matrix) → `artifact-verify` → gate. **Manually reject** the gate. Confirm: no artifacts uploaded to release, no signing secrets consumed.
- Throwaway tag `v0.0.0-split-test-2`: same flow, **approve** the gate. Validates `sign-*` jobs end-to-end: smoke passes, real signing produces valid artifacts, `publish` creates the release.
- Verify the signed assets:
  - `signtool verify /pa /v <installer.exe>` (Windows) — valid cert chain back to HartyeTech.
  - `codesign --verify --deep --strict --verbose=2 Snapper Keeper.app` (macOS) — `valid on disk` + `satisfies its Designated Requirement`.
  - `minisign -V -p <pubkey from tauri.conf.json> -m <installer.exe> -x <installer.exe.sig>` — `Signature and comment signature verified`.
- Delete test releases + tags after.

### PR-3 (audit + SBOM)

- `audit.yml` triggered via `workflow_dispatch` for testing (don't wait for 06:17 UTC):
  - Empty advisory case (current repo state) → no issues opened, no errors.
  - Synthetic advisory case: feed the script a fake JSON via test harness. Verify one issue opened with right labels and body.
  - Already-open advisory: re-run; confirm no duplicate.
  - Resolved advisory: remove from input; verify open issue gets a comment and closes.
- Test the script locally via `node` against fixture inputs under `.github/scripts/__tests__/`. Stubbed `octokit` mock.
- Throwaway release tag for SBOM generation. Verify `sbom.cdx.json` is valid CycloneDX (`pnpm dlx @cyclonedx/cyclonedx-cli@<pinned> validate --input-file sbom.cdx.json`).

### Verification commands per PR (pre-merge gate)

```bash
pnpm lint && pnpm typecheck
cargo fmt -- --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
cargo test --workspace --exclude snapper-keeper-app
# PR-2 only:
cargo build -p snk-smoke-target
```

Throwaway tag teardown:

```bash
gh release delete v0.0.0-split-test-1 --yes
git push origin :v0.0.0-split-test-1
git tag -d v0.0.0-split-test-1
```

## 10. Open decisions deferred to the implementation plan

- **Exact pinned versions**: `dotnet sign` CLI version, `cargo-audit` version, `cargo-cyclonedx` version, `@cyclonedx/cyclonedx-npm` version, `@cyclonedx/cyclonedx-cli` version, `minisign` apt/brew/choco versions per platform, Tesseract chocolatey package version + expected SHA256. Lookup current stable at plan time; commit the numbers.
- **Exact SHAs per pinned action**: resolved at PR-1 authoring time via `gh api`.
- **Whether `snk-smoke-target` lives in `tests/fixtures/smoke-target/` or `crates/snk-smoke-target/`**: spec says `tests/fixtures/`; plan should confirm workspace-membership ergonomics (excluding from default `cargo test` runs).
- **`audit:auto` label color + final description text**: minor; pick at PR-3 authoring.
- **SBOM merge tool choice**: `@cyclonedx/cyclonedx-cli` is the proposed merger; if it doesn't satisfy the merge use case, fall back to manual jq.
- **Whether to delete `Snapper.Keeper_${ARCH}.app.tar.gz.sig` from the build job's responsibility** (today the rename happens in build; in this design it moves to sign — the per-arch rename is the sign job's responsibility since the `.sig` doesn't exist until after minisign).

## 11. Prerequisites (Eric's action items before PR-2)

1. **Create the `production-release` environment** in repo Settings → Environments:
   - Name: `production-release`
   - Required reviewers: `ehartye`
   - Deployment branches and tags: Selected branches and tags → add `Tags` → `v*`
   - Wait timer: 0
   - Environment secrets: none (this design uses repo-level secrets)
2. **Confirm `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets are populated** at repo level (already are; sanity-check post-environment-creation since environment scoping is a foot-gun).

Both can be done by Eric in ~5 minutes. PR-1 (`#30` SHA pinning) doesn't depend on either; it can land first while the environment is being set up.

## 12. References

- Issue bodies: [#28](https://github.com/ehartye/snapper-keeper/issues/28), [#29](https://github.com/ehartye/snapper-keeper/issues/29), [#30](https://github.com/ehartye/snapper-keeper/issues/30), [#46](https://github.com/ehartye/snapper-keeper/issues/46), [#75](https://github.com/ehartye/snapper-keeper/issues/75)
- Source review: [synthesis.md § AC5, CP3, NI7, O-U4, T-U1, A-U8, O-U3](../reviews/2026-05-24-prerelease/synthesis.md)
- Current pipeline: [`.github/workflows/release.yml`](../../../.github/workflows/release.yml), [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml), [`docs/release-signing.md`](../../release-signing.md)
- Project notes: [`CLAUDE.md`](../../../CLAUDE.md) (one Tauri plugin per feature; >500 LoC red flag; commit conventions)
- Persistent notes (`MEMORY.md`): keychain timeout, Tauri 2 updater bundle artifacts, Git Bash MSYS path conversion, Windows UAC installer-detection heuristic
