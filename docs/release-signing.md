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

## Tesseract bundling

Windows release builds ship a copy of Tesseract OCR (engine + DLLs + `eng.traineddata`) alongside the app so users don't need a separate install. The release workflow runs `choco install tesseract` on the Windows runner and copies `C:\Program Files\Tesseract-OCR\` into `app/src-tauri/resources/tesseract/` before `tauri build`. The bundler then ships those files inside the installer.

At runtime, `snk-ocr/sidecar.rs` resolves tesseract in this order:

1. `SNK_TESSERACT_PATH` env var (override for dev/debug)
2. Bundled location (`<resource_dir>/tesseract/tesseract.exe`)
3. System `PATH`
4. Common install locations per OS

macOS bundles are not yet self-contained for OCR — users currently need `brew install tesseract`. This is because Homebrew's tesseract binary references absolute `/opt/homebrew/lib/...` dylib paths, and bundling requires running `install_name_tool` to rewrite each path to `@executable_path/../Frameworks/...`. To be added later.
