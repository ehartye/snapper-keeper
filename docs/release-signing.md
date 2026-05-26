# Release Signing Setup

## Ed25519 updater key pair

The Tauri updater uses Ed25519 signatures to verify update payloads. The private key signs `latest.json` during CI; the public key is embedded in `tauri.conf.json`.

### Generate the key pair

```bash
pnpm tauri signer generate -w ~/.tauri/snapper-keeper.key
```

This creates:
- `~/.tauri/snapper-keeper.key` — private key (password-protected)
- `~/.tauri/snapper-keeper.key.pub` — public key

### GitHub Actions secrets

Add these secrets to the repository (`Settings > Secrets and variables > Actions`):

| Secret | Value | Used by |
|--------|-------|---------|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `~/.tauri/snapper-keeper.key` | `release.yml` — signs update bundles |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password used during key generation | `release.yml` — unlocks private key |

### macOS signing secrets

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email for `notarytool` |
| `APPLE_PASSWORD` | App-specific password for `notarytool` |
| `APPLE_TEAM_ID` | 10-character team ID |

### Windows signing secrets (Azure Artifact Signing)

Windows code signing uses [Azure Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/overview) (formerly "Trusted Signing" / "Azure Code Signing"). The CA/Browser Forum's June 2023 hardware-storage mandate killed the downloadable `.pfx` path for new OV code-signing certs from every public CA; cloud-HSM services like Azure Artifact Signing are the modern replacement.

The release workflow installs the [`dotnet sign` CLI](https://github.com/dotnet/sign) on the Windows runner and invokes it via Tauri's `TAURI_WINDOWS_SIGN_COMMAND` hook for each produced artifact (raw `.exe` and NSIS installer). The CLI authenticates via `DefaultAzureCredential`, which reads the service-principal env vars set on the build step.

| Secret | Value | Source |
|--------|-------|--------|
| `AZURE_TENANT_ID` | Entra ID tenant GUID | Azure Portal → top-right account avatar |
| `AZURE_CLIENT_ID` | Service principal `appId` | Output of `az ad sp create-for-rbac` |
| `AZURE_CLIENT_SECRET` | Service principal `password` | Same output; shown once, save to password manager |
| `AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME` | Artifact Signing Account name | Azure Portal → Artifact Signing Accounts |
| `AZURE_ARTIFACT_SIGNING_CERT_PROFILE` | Certificate profile name under that account | Account → Certificate profiles |

The service principal must have the **`Artifact Signing Certificate Profile Signer`** RBAC role assigned at the certificate-profile scope. Example (run once, replace bracketed values):

```bash
az ad sp create-for-rbac \
  --name "gh-snapper-keeper-signer" \
  --role "Artifact Signing Certificate Profile Signer" \
  --scopes "/subscriptions/<SUB_ID>/resourceGroups/<RG>/providers/Microsoft.CodeSigning/codeSigningAccounts/<ACCOUNT>/certificateProfiles/<PROFILE>"
```

Identity validation (Public Trust → Individual) must be completed in the Azure Portal before a Certificate Profile can be created — the CLI cannot drive this step.

## Updater endpoint and the first-tag-must-be-plain-SemVer rule

The Tauri updater's endpoint in `tauri.conf.json` is:

```
https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json
```

GitHub's `releases/latest/` redirect resolves to **the most recent non-prerelease release**. Releases marked as `prerelease: true` are excluded from this resolution.

**Operational consequences:**

- The **first user-facing tag must be a plain SemVer** (`v0.1.0`, NOT `v0.1.0-beta.1`). If the very first release is a prerelease, `releases/latest/` returns 404 and the updater fails silently for every installed client.
- Subsequent prerelease tags (e.g. `v0.2.0-rc.1`) are fine — they don't disturb the `/latest/` pointer; existing clients on `v0.1.0` simply don't see them as available updates.
- Once a non-prerelease tag exists, the updater path is self-healing — any future non-prerelease tag becomes the new `/latest/` target.

**Failure modes to know:**

1. **Endpoint 404** — typically means no non-prerelease release exists yet, OR GitHub Releases is temporarily down. The updater logs the network error and retries on the next 24h cycle. No auto-rollback; users stay on their current version.
2. **`latest.json` parses but `platforms{}` is empty** — usually a workflow bug where `createUpdaterArtifacts: true` is off in `tauri.conf.json` or the publish-release job's `find` doesn't match the produced filenames. See [`reference_tauri2_updater_artifacts`](../.research/) for the historical iteration on this.
3. **Signature verification failure** — Ed25519 mismatch between the embedded `pubkey` in `tauri.conf.json` and the `TAURI_SIGNING_PRIVATE_KEY` used to sign. Updater rejects the manifest and ceases auto-checks for this process. Surface via Settings → About (planned).

A redundant fallback endpoint (e.g. a GitHub Pages mirror of `latest.json`) is a future improvement tracked in issue #43; the single-endpoint approach is acceptable for v0.1.0 while we observe operational behavior.
