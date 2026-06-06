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

## Data retention

All retention is local and bounded by your own activity — nothing expires on a
server because there is no server.

- **Captures** (screenshots and their annotations) are kept until you delete
  them. Deleting moves a capture to Trash; emptying Trash removes it and its
  image files permanently. There is no automatic expiry.
- **OCR text** is stored alongside its capture and is removed when that capture
  is permanently deleted.
- **Clipboard history** keeps your most-recent *unpinned* entries (currently up
  to 200) and evicts older unpinned entries as new ones arrive; *pinned* entries
  are kept until you unpin or delete them. <!-- CLAIM: privacy-md/clipboard-retention -->
  Clipboard content the OS marks sensitive (e.g. password-manager copies) is
  never stored.

Data at rest is stored unencrypted in a local SQLite database, relying on your
operating system's user-account isolation. To remove everything, delete the
application data directory shown in Settings → About.

---

Source code: <https://github.com/ehartye/snapper-keeper>
Support: <https://github.com/ehartye/snapper-keeper/issues>
Contact: <owner@hartye.com>
