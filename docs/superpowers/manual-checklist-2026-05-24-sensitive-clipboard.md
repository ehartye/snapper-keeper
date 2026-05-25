# Manual smoke checklist — sensitive-clipboard exclusion

End-to-end verification for the sensitive-clipboard feature on `feature/sensitive-clipboard`. The automated gates (cargo + vitest + lint + typecheck) are already green; this checklist covers the live-clipboard, live-UI scenarios that benefit from a human operator at the keyboard.

Allow ~10 minutes. Requires an interactive desktop session (not SSH — see `CLAUDE.md` for the Windows OpenSSH / window-station limitation).

## Preconditions

- [ ] Quit any **installed** snapper-keeper build before starting. The installed prod app's watcher and the dev build will both race for the clipboard listener; only one should be alive.
- [ ] On Windows: 1Password (or another password manager that uses the `CanIncludeInClipboardHistory = 0` clipboard flag — Bitwarden, KeePassXC also work) should be installed and signed in.
- [ ] On macOS: a password manager that uses `NSPasteboardTypeConcealed` / `org.nspasteboard.ConcealedType` — 1Password 8, Bitwarden, KeePassXC.
- [ ] Have one "control" app open from which you'll copy ordinary text (e.g., Notepad on Windows, Notes on macOS, or VS Code).
- [ ] Be ready to ignore whatever is currently in your clipboard — the test sequence will overwrite it multiple times.

## Launch

1. From the worktree root:
   ```
   pnpm --filter @snk/app tauri dev
   ```
2. Wait for the snapper-keeper main window to appear and the tray icon to register.

## Test sequence

### 1. Baseline — ordinary text from a regular app appears in the popup
1. Focus the control app (Notepad / Notes / VS Code).
2. Select some text and copy it (Ctrl+C / Cmd+C).
3. Open the snapper-keeper clipboard popup (default hotkey: Ctrl+Shift+V on Windows, Cmd+Shift+V on macOS).

**Expected:** the just-copied text appears as the topmost item in the popup.
**Regression signal:** popup is empty or shows an older entry — the watcher isn't observing the clipboard.

### 2. OS-level sensitive flag — password manager copy does NOT appear
1. Open your password manager.
2. Find any entry and click its "copy password" button (do **not** type the password into a plain text field — use the manager's copy action, which sets the OS concealed flag).
3. Open the snapper-keeper clipboard popup.

**Expected:** the password does **NOT** appear in the popup. The topmost entry should still be the text from step 1.
**Regression signal:** the password is in the popup. This means either the OS flag isn't being detected (sensitivity probe broken) OR the watcher is recording before the flag is inspected (worker_step ordering broken). Send back to the platform implementer for that OS.

### 3. Add-from-frontmost flow — bind a regular app to the blocklist
1. Open the Settings window (right-click tray icon → Settings, or whatever the project's Settings shortcut is).
2. Scroll to the "Excluded apps" panel (between Clipboard and OCR sections).
3. Focus the control app from step 1 (VS Code, Notepad, etc.) for ~1 second to make sure it's the OS-level frontmost app.
4. Click back to Settings and click **"+ Add from frontmost app"**.

**Expected:** a confirmation modal appears showing the control app's identifier:
   - Windows: `Code.exe` or `notepad.exe`
   - macOS: `com.microsoft.VSCode` or `com.apple.Notes`
   The display name should be human-readable (`Visual Studio Code`, `Notepad`, etc.).
**Regression signal:** modal shows wrong app, blank identifier, or doesn't appear. Means `detect_frontmost_app` IPC or the OS-level frontmost lookup is broken.

5. Click **Add** in the modal.

**Expected:** the entry appears in the "Excluded apps" list with the identifier and kind.

### 4. Blocklist takes effect — newly blocked app's copy is filtered
1. Focus the control app (now in the blocklist).
2. Copy a fresh, distinct string (e.g., "blocklist-test-12345").
3. Open the snapper-keeper popup.

**Expected:** "blocklist-test-12345" does **NOT** appear in the popup. The most recent entry should be older than the copy you just made.
**Regression signal:** the blocked string is in the popup. Means the blocklist match isn't running OR `source_app::current()` returned a different identifier than the one in the blocklist.

### 5. Remove from blocklist — events resume
1. Back in Settings → "Excluded apps", click the `×` button next to the control-app entry.
2. Focus the control app again.
3. Copy another distinct string (e.g., "unblocked-test-67890").
4. Open the popup.

**Expected:** "unblocked-test-67890" appears at the top.
**Regression signal:** still filtered. Means `persist()`'s `invalidateQueries` isn't triggering a refetch, OR the watcher cached the old blocklist and didn't re-read settings.

### 6. Manually-typed bundle ID / exe filename works as well
1. In Settings → "Excluded apps", click **"+ Add app…"**.
2. Pick the appropriate kind (macos_bundle_id or windows_exe).
3. Enter an identifier for the password manager (e.g., `com.1password.1password8` for macOS, `1Password.exe` for Windows). Use a real one from the password manager you tested in step 2.
4. (Optional) give it a display name.
5. Click **Add**.

**Expected:** entry appears in the list. Now both the OS flag (step 2 path) AND the blocklist match would filter it; the manually-added entry provides defense-in-depth and protects against password managers that don't set the OS flag.

### 7. Duplicate guard — adding the same entry twice is blocked
1. In Settings → "Excluded apps", click **"+ Add app…"** again.
2. Enter the exact same identifier and kind you added in step 6.
3. Click **Add**.

**Expected:** an inline error appears: "Already in the list." The modal does not close.
**Regression signal:** the duplicate is added, or the modal closes silently. Means the dup-check predicate is broken.

## Shutdown

- [ ] Close the Settings window (the X just hides it — the webview stays alive).
- [ ] Quit `tauri dev` (Ctrl+C in the terminal).
- [ ] Restart the installed prod snapper-keeper if you want it running again.

## Reporting

If any step's expected behavior is missing, file an issue referencing this checklist's step number and which OS you ran it on. If all 7 pass, the manual smoke gate is satisfied and the branch is ready for PR.
