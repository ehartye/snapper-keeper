# Release Strategy

**Audience:** Claude (or any agent) cutting releases of snapper-keeper. For the human-facing canary→stable lifecycle, anti-patterns, and rollback procedure, see [`release-process.md`](release-process.md) — this doc is the operational playbook (how to drive the pipeline end-to-end, what breaks, how to debug) that complements it.

Also read: [`MEMORY.md`](../../../.claude/projects/C--Users-ehart-OneDrive-Documents-repos-snapper-keeper/memory/MEMORY.md) for field-tested gotchas (env-gate API, BSD/GNU base64 portability, Git Bash PATH, cdxgen for SBOMs, Tauri `--config` overlay traps).

---

## TL;DR

```
tag (vX.Y.Z) → verify-pubkey → build (matrix, NO secrets) → artifact-verify
            → env gate (production-release, manual or auto-approve)
            → sign-mac-arm / sign-mac-x64 / sign-win-x64 (scoped secrets)
            → publish-release (gh release create + CycloneDX SBOM upload)
```

A canary cut takes **~30 min end-to-end**. The slowest leg is `build (macos-15-intel, ...)` at ~17 min; the sign jobs together take ~3 min once the gate clears.

You can drive the whole thing autonomously if you have:
- `gh auth` as a repo collaborator + `production-release` environment reviewer
- Explicit user authorization to auto-approve env gates
- The `production-release` environment exists with a deployment policy `name:"v*"` `type:"tag"` (NOT `type:"branch"` — that's how PR #92 first failed)

---

## Strategy

### Canary-by-default

Every release ships as `prerelease: true` first. No exceptions. The auto-updater's `/releases/latest` redirect ignores prereleases, so any user on the stable channel won't auto-pick up a canary. Manual install is the only way to use a canary; that's intentional.

A canary becomes stable only after:
1. ~1–2 weeks of daily soak (you using it; if you don't actually use it, you're not soaking it).
2. A proven `vN-1 → vN` auto-update path (install older version, point updater at the new tag, watch the upgrade succeed).

See [`release-process.md` §Canary phase](release-process.md#canary-phase) for the human-facing version.

### Never force-push a release tag

If a canary ships broken, **bump the version** and cut a new tag. Don't `git push --force` the old one. The auto-updater hashes the binaries at the tag — if you replace bytes under a tag, any client that already downloaded `latest.json` has a signature mismatch and the upgrade fails.

This is in [`release-process.md` anti-patterns](release-process.md#anti-patterns) too; flagging again because it's the most expensive mistake to recover from. The v0.1.1 → v0.1.2 cycle in 2026-05-25 was exactly this: v0.1.1 shipped with the `.preview.png` scope bug, fix went in PR #122, v0.1.2 was cut as the replacement, v0.1.1's notes redirect to v0.1.2.

### "Latest" pointer hygiene

GitHub's `/releases/latest` API returns the highest non-prerelease tag. If your only releases are canaries, `/releases/latest` returns 404 — that's *correct* and what you want. The stable channel is intentionally empty until you promote.

If you ever see a non-prerelease tag that shouldn't be the "latest" (e.g., an old pre-hardening v0.1.0 that snuck through), mark it prerelease retroactively:

```bash
gh release edit v0.1.0 --prerelease
```

Add a note to the body explaining why (don't leave readers confused about a tag that quietly moved channels). 2026-05-25 example: v0.1.0 was tagged before the hardening cluster shipped, then marked prerelease so it stopped being the "latest" pointer.

### Version semantics

- **Patch bumps (`vX.Y.Z+1`)** for fix-only canaries. The v0.1.1 → v0.1.2 cut was a patch bump for a single regression fix.
- **Minor bumps (`vX.Y+1.0`)** when meaningful features land.
- **Major bump (`vX+1.0.0`)** for breaking changes (config schema break, plugin API rename, DB migration that can't reverse).

The current `tauri.conf.json` and `Cargo.toml` versions should always reflect the *next* canary you'd cut. Bump them in the PR that prepares for the cut, not retroactively.

---

## Operational mechanics: cutting a canary, end-to-end

The full sequence, with concrete commands. Substitute `X.Y.Z` for the new version throughout.

### 1. Create a worktree

Per CLAUDE.md convention, sibling location:

```bash
git -C <primary-repo> worktree add C:/Users/ehart/repos/snapper-keeper-worktrees/release-vX.Y.Z-bump -b release/vX.Y.Z-bump main
```

Don't edit version files in the primary workspace — it shares the lockfile-state with whatever branch is currently checked out there, and you'll get cross-branch contamination.

### 2. Bump version in three files

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/release-vX.Y.Z-bump
sed -i 's/"version": "<OLD>"/"version": "X.Y.Z"/g' app/package.json app/src-tauri/tauri.conf.json
sed -i 's/^version = "<OLD>"$/version = "X.Y.Z"/g' app/src-tauri/Cargo.toml
pnpm install         # regen pnpm-lock.yaml
cargo update -p snapper-keeper-app   # regen Cargo.lock (only the app's entry)
```

All three files must change; the Tauri bundler reads `tauri.conf.json`, the bundler-emitted artifacts include the version from `Cargo.toml`, and `pnpm` uses `package.json`.

### 3. PR + auto-merge

```bash
git add app/package.json app/src-tauri/Cargo.toml app/src-tauri/tauri.conf.json Cargo.lock pnpm-lock.yaml
git commit -m "chore(release): bump version to X.Y.Z"
git push -u origin release/vX.Y.Z-bump
gh pr create --title "chore(release): bump version to X.Y.Z" --body "..."
```

Watch CI with a Monitor (see [Monitor pattern](#monitor-pattern-for-auto-merge) below). When green, auto-merge:

```bash
gh pr merge <PR-NUMBER> --squash --delete-branch --subject "..." --body "..."
```

Note: `gh pr merge` will error locally with `'main' is already used by worktree at ...` — that's a benign Windows worktree-sync issue. The remote merge succeeds; just verify with `gh pr view <PR-NUMBER> --json state`.

### 4. Sync main, push the tag

```bash
git -C <primary-repo> pull --ff-only
git -C <primary-repo> tag vX.Y.Z
git -C <primary-repo> push origin vX.Y.Z
```

Tag must be pushed from `main` after the version-bump merge lands — otherwise the artifact filenames embed the wrong version.

### 5. Watch the release run

```bash
gh run list --workflow=release.yml --limit=1 --json databaseId,headBranch,status
# Note the databaseId; that's your RUN
gh run watch <RUN-ID> --exit-status
```

Or wire up a Monitor that also auto-approves the env gate (template in [`MEMORY.md` → reference-github-env-gate-auto-approve](../../../.claude/projects/C--Users-ehart-OneDrive-Documents-repos-snapper-keeper/memory/reference_github_env_gate_auto_approve.md)).

### 6. Approve the env gate

When the sign-* jobs reach `status: "waiting"`, the gate is up. Approve:

```bash
RUN=<run-id>
ENV_ID=15753639452  # production-release; check via /environments/production-release if it changes
echo '{"environment_ids":['$ENV_ID'],"state":"approved","comment":"..."}' \
  | gh api -X POST repos/ehartye/snapper-keeper/actions/runs/$RUN/pending_deployments --input -
```

**Critical syntax:** `--input -` with a JSON body. `gh api -f environment_ids:=[...]` does NOT work — gh's `:=` syntax rejects array values for this endpoint.

### 7. Post-release: mark prerelease + write notes

Once the release publishes, it's `isPrerelease: false` by default. Flip it:

```bash
gh release edit vX.Y.Z --prerelease --notes "$(cat <<'EOF'
## vX.Y.Z — canary

[fill in: scope of changes, link to PRs, canary policy reminder]

### Verifying

- macOS: `codesign --verify --deep --strict --verbose=2 Snapper.Keeper_<arch>.app.tar.gz`
- Windows: `signtool verify /pa /v Snapper.Keeper_X.Y.Z_x64-setup.exe`
- Updater: minisign-verify each `.sig` against the pubkey in `app/src-tauri/tauri.conf.json`
EOF
)"
```

If you're replacing a previous broken canary, also update the previous tag's notes to point forward (`gh release edit <prev-tag> --notes "...replaced by vX.Y.Z..."`).

### 8. Clean up

```bash
git -C <primary-repo> worktree remove --force C:/Users/ehart/repos/snapper-keeper-worktrees/release-vX.Y.Z-bump
rm -rf C:/Users/ehart/repos/snapper-keeper-worktrees/release-vX.Y.Z-bump   # Windows file-lock fallback
```

`git worktree remove` often fails on Windows because something in `node_modules` is still mapped; `rm -rf` always works after the action stops touching it.

---

## Monitor pattern for auto-merge

Long-running CI + multi-phase release workflows are easier to drive with one Monitor that emits progress events and exits at completion. Template that handles the full version-bump-PR → tag → release → mark-prerelease pipeline as a single command (see PR #124 monitor for a working example):

```bash
PR=<pr-num>
ENV_ID=15753639452
prev=""
phase="ci"
RUN=""
approved=0
while true; do
  ts=$(date +%H:%M:%S)

  if [ "$phase" = "ci" ]; then
    state=$(gh pr view $PR --json mergeStateStatus,state --jq '"\(.state)/\(.mergeStateStatus)"' 2>/dev/null)
    [ -z "$state" ] && { sleep 20; continue; }
    case "$state" in MERGED/*) phase="tag" ;; esac
    [ "$phase" = "ci" ] && {
      pass=$(gh pr checks $PR --json bucket --jq '[.[] | select(.bucket == "pass")] | length' 2>/dev/null)
      fail=$(gh pr checks $PR --json bucket --jq '[.[] | select(.bucket == "fail")] | length' 2>/dev/null)
      pending=$(gh pr checks $PR --json bucket --jq '[.[] | select(.bucket == "pending")] | length' 2>/dev/null)
      sig="pr_pass=$pass pr_fail=$fail pr_pending=$pending pr_state=$state"
      [ "$sig" != "$prev" ] && { echo "[$ts] $sig"; prev="$sig"; }
      [ "${fail:-0}" -gt 0 ] && { echo "[$ts] PR FAILED"; exit 1; }
      if [ "${pending:-1}" -eq 0 ] && [ "${pass:-0}" -gt 0 ]; then
        gh pr merge $PR --squash --delete-branch ... > /tmp/merge.out 2>&1
        phase="tag"
      fi
    }
  fi

  if [ "$phase" = "tag" ]; then
    cd <primary-repo> && git fetch origin --quiet && git pull --ff-only > /dev/null 2>&1
    git tag vX.Y.Z 2>/dev/null
    git push origin vX.Y.Z 2>/dev/null
    sleep 5
    RUN=$(gh run list --workflow=release.yml --limit=1 --json databaseId,headBranch \
          --jq '.[] | select(.headBranch == "vX.Y.Z") | .databaseId')
    [ -n "$RUN" ] && { phase="release"; prev=""; }
  fi

  if [ "$phase" = "release" ]; then
    status=$(gh run view "$RUN" --json status --jq '.status' 2>/dev/null)
    if [ "$status" = "completed" ]; then
      concl=$(gh run view "$RUN" --json conclusion --jq '.conclusion' 2>/dev/null)
      [ "$concl" = "success" ] && {
        gh release edit vX.Y.Z --prerelease > /dev/null 2>&1
        # ... update notes here
      }
      exit 0
    fi
    waiting=$(gh run view "$RUN" --json jobs \
              --jq '.jobs[] | select(.name | startswith("sign-")) | select(.status == "waiting") | .name' 2>/dev/null | head -1)
    if [ -n "$waiting" ] && [ $approved -eq 0 ]; then
      echo '{"environment_ids":['$ENV_ID'],"state":"approved","comment":"..."}' \
        | gh api -X POST repos/ehartye/snapper-keeper/actions/runs/$RUN/pending_deployments --input - > /dev/null 2>&1
      approved=1
    fi
    sleep 45
  fi

  sleep 5
done
```

Key constraints:
- Use `gh ... --jq '...'` (gh's built-in jq), NOT `... | jq ...` — standalone `jq` isn't on the local PATH (caught in PR #98 work).
- Polling cadence ~30s for active phases, ~45s during long Rust compiles. Going faster doesn't help (build times are determined by the runners).
- Emit only on state changes (`if [ "$sig" != "$prev" ]`); otherwise the notification stream gets too noisy.

---

## Debugging: symptom → suspect map

When something goes wrong in the pipeline, this is the first place to look:

| Symptom | Likely cause | What to check |
|---|---|---|
| `verify-pubkey` job fails with `base64: invalid input` | Git Bash on Windows CR-mangling the secret env var | `scripts/verify-pubkey.sh` uses `tr -d '\r' \| base64 -d` (NOT `base64 -di`; that breaks BSD) — see PR #98 |
| `build` matrix fails with `A public key has been found, but no private key` | Tauri's `--config '{"plugins":{"updater":{"pubkey":""}}}'` overlay treats `""` as configured | Use `bundle.createUpdaterArtifacts: false` instead; drop pubkey override from overlay — see PR #92 |
| `build` matrix fails with `failed to bundle project: program path has no file name` | Tauri's `--config '{"bundle":{"windows":{"signCommand":""}}}'` tries to exec empty command | `jq del .bundle.windows.signCommand` from `tauri.conf.json` on Windows runners before build — see PR #92 |
| `sign-win-x64` smoke fails with `signtool: command not found` | Git Bash PATH doesn't include the Windows SDK | Drop `signtool verify` from the smoke script; `sign code` exit code is the validation. Real installer verification happens via PowerShell `Get-AuthenticodeSignature` later in the same job — see PR #92 |
| `publish-release` fails at `gh release create` with `no matches found for 'artifacts/**/*.dmg'` | Non-interactive bash treats `**` as `*`; `merge-multiple: true` flattens artifacts to `artifacts/X.dmg` | `shopt -s globstar nullglob` before the `gh release create` line — see PR #92 |
| `publish-release` SBOM step fails with `npm-ls exited with errors: 1` | `cyclonedx-npm` calls `npm ls` internally; doesn't work with pnpm | Use `@cyclonedx/cdxgen -t pnpm` instead — see PR #98 |
| `publish-release` SBOM step produces SBOM with only npm components (no Rust) | `cargo cyclonedx --all` produces no findable files on workspace-root-only `Cargo.toml` | Use `@cyclonedx/cdxgen -t cargo` instead — see PR #98 |
| `publish-release` fails at pnpm/action-setup with `Multiple versions of pnpm specified` | `with: version: 9` input AND `packageManager: pnpm@X` in package.json | Drop the `with: version:` input; let `packageManager` drive — see PR #113 |
| Sign jobs run 0 steps in ~1 sec and fail with no useful message | `production-release` env's deployment policy is `type:"branch"`, not `type:"tag"` | `gh api repos/.../environments/production-release/deployment-branch-policies` → POST a `type:"tag"` policy, DELETE the branch one — see PR #92 |
| Capture saves but library thumbnails show blank/black | A new file path is being written outside the assetProtocol allow scope | Check the path against `tauri.conf.json` `security.assetProtocol.scope.allow`; move the file into a scoped subdir (`captures/`, `clipboard/`, `annotations/`) — see PR #122 |

Beyond this map: `cd <worktree> && gh api repos/ehartye/snapper-keeper/actions/jobs/<JOB_ID>/logs 2>&1 | tail -60` is the universal "what actually failed" tool. The `gh run view --log-failed` flag only works after the *whole run* completes — useless mid-flight; the per-job logs API is the workaround.

---

## Anti-patterns (don't)

| Anti-pattern | Why it's bad | Do instead |
|---|---|---|
| **Force-push a release tag** | Breaks auto-updater for any client that already downloaded `latest.json` (signature mismatch on next check) | Bump version, cut new tag |
| **Tag-and-pray** (cut stable without canary soak) | The release pipeline is the project's most fragile path; canary is the only check for whether the auto-updater works | See [`release-process.md` §Canary phase](release-process.md) |
| **Promote a canary you didn't actually use** | Bugs hide in long-running state, daily-use friction, and rare interactions. Soak is a calendar test as much as a usage test | Actually use the canary for ~1–2 weeks |
| **Edit `tauri.conf.json` directly in the primary workspace** | The primary workspace is shared across branches; the lockfile state gets confused | Make a worktree off main |
| **`gh release delete` on a release with non-zero download counts** | Breaks anyone who already downloaded the artifacts (signatures mismatch any updates) | Bump version and cut a new tag |
| **Trust GitHub's `latest` calculation** | If you have non-prerelease tags older than your current canary set, `/releases/latest` points at an old one (probably wrong) | Mark all non-canary releases as prerelease while in canary phase |
| **Reuse a worktree across PRs** | Branch state contaminates the next PR; the prior commits sneak into the merge | One worktree per branch; remove on merge |
| **Add a new file path the WebView reads without updating the assetProtocol scope** | The scope check fails silently, the URL returns nothing, the page renders a fallback (often black or blank) | New paths go inside `captures/`, `clipboard/`, or `annotations/`. If a different subdir is genuinely needed, add to the allow list AND remember the deny list overrides it for SQLite + logs + backups |
| **`pnpm/action-setup@v6` with `with: version: 9`** | v6 errors when both are set; subtle bug that hits each pnpm-action-setup invocation independently | Use `packageManager` field in package.json; drop `with: version:` from the workflow |
| **Use `base64 -di` for portability** | `-i` is GNU's `--ignore-garbage` but BSD's `--input-file` — opposite meanings | Use `tr -d '\r' \| base64 -d` instead |

---

## Plan deviations during execution

When you find a real bug in the plan or in a process this doc describes, raise it to the user (the team-lead in plan-as-source-of-truth terms) **before** applying the fix. Then:

1. Update the plan / this doc in place so the audit trail is "plan was fixed because X" not "code diverged from plan."
2. Apply the fix.
3. Plan-fix and implementation can bundle in one commit, or split if the plan-fix is substantial.

See [`feedback_plan_as_source_of_truth.md`](../../../.claude/projects/C--Users-ehart-OneDrive-Documents-repos-snapper-keeper/memory/feedback_plan_as_source_of_truth.md) in memory.

This doc *itself* is plan-source-of-truth. If you hit a release-pipeline behavior that contradicts what's written here, that's a bug in the doc; surface it, fix the doc, ship the working procedure.

---

## Quick reference

| Thing | Value |
|---|---|
| `production-release` env ID | `15753639452` |
| Release workflow | `.github/workflows/release.yml` |
| Tag trigger pattern | `v*` |
| Average canary cycle | ~30 min from tag-push to release-published |
| Slowest matrix leg | `build (macos-15-intel, ...)` at ~17 min |
| Sign jobs (post-gate) | ~3 min combined |
| SBOM step location | `publish-release` job, after `gh release create` |
| Pubkey (in `tauri.conf.json`) | `plugins.updater.pubkey` — base64 minisign pubkey |
| Private key (in env) | `TAURI_SIGNING_PRIVATE_KEY` (base64 minisign privkey) + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| Apple secrets | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| Azure secrets | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` |

| Cluster PRs (release pipeline) | Scope |
|---|---|
| #90 | SHA-pin actions + sidecar bundling + replace softprops |
| #92 | Build/sign split + env gate + smoke vehicle |
| #98 | Nightly audit + per-release SBOM (cdxgen) |
| #109 | vite/esbuild advisory bumps |
| #110 | Node 24 action majors (release.yml + audit.yml) |
| #102 | Node 24 action majors (ci.yml) |
| #112 | v0.1.1 version bump |
| #113 | pnpm-action-setup v6 version conflict fix |
| #122 | `.preview.png` inside assetProtocol scope |
| #124 | v0.1.2 version bump |

---

## When to update this doc

- A new failure mode appears that's not in the symptom→suspect map. Add a row.
- The pipeline structure changes (new job, new gate, secret rotation). Update the TL;DR + Quick Reference.
- A new anti-pattern is identified through pain. Add a row.
- The "Average canary cycle" timing drifts by >20% (probably means a major dep or runner changed). Update + investigate.

Don't let this drift; future agents (including future you) lose the field-tested knowledge if it isn't kept current.
