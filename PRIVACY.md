# Snapper Keeper Privacy Policy

Last updated: 2026-05-23

Snapper Keeper does not collect, transmit, or store any personal information
about you.

Specifically:

- We do not use analytics, crash reporters, or telemetry of any kind.
- We do not have accounts, sign-in, or any concept of users.
- The application does not transmit your captures, clipboard contents, OCR
  text, or any usage data over the network.
- All data — captures, clipboard history, OCR text, settings — is stored
  locally on your device. <!-- CLAIM: privacy-md/local-only-storage -->
- Password-manager copies (clipboard content marked sensitive by the OS) are
  filtered before they reach the local clipboard history. <!-- CLAIM: privacy-md/sensitive-clipboard -->


The application makes ONE category of network request, and only in the
GitHub Releases edition:

1. **Update checks:** When the app starts and once every 24 hours
   thereafter, it contacts `github.com` to check whether a newer release is
   available. This request contains your application's current version
   number and your IP address (as with any HTTP request). No personally
   identifiable information is transmitted. You can disable update checks
   in Settings.

The **Microsoft Store edition makes zero network requests**. The in-app
updater is compiled out; the Microsoft Store handles version delivery via
its listing page.

The application does NOT:

- Read or upload files outside the application's own data directory unless
  you explicitly drag them in.
- Use the clipboard contents for anything other than displaying them in
  the local clipboard history.
- Use the OCR text for anything other than local search indexing.
- Share data with third parties (there are no third parties).

This policy applies to all distribution channels — GitHub Releases,
Microsoft Store, winget, and Homebrew Cask.

---

Source code: <https://github.com/ehartye/snapper-keeper>
Support: <https://github.com/ehartye/snapper-keeper/issues>
Contact: <owner@hartye.com>
