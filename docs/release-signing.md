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

### Windows signing secrets

| Secret | Value |
|--------|-------|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` code-signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` |
