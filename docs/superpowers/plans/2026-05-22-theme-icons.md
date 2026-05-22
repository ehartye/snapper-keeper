# Theme Icons Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Generate dedicated 256×256 tray + window PNG icons for the 4 newest theme families (`wabi-sabi`, `riso`, `constructivist`, `atomic`) so the tray icon matches the active theme instead of falling through to `sk-holo.png`.

**Architecture:** Extend the existing procedural recipe in `app/src-tauri/icons/generate-sk.ps1`. One small refactor (optional ghost-text parameters on `New-SkIcon` for the riso misregistration effect), then 4 new `New-SkIcon` invocations. Bundle the 4 new PNGs into the Rust binary via `include_bytes!` and add match arms in `tray_icon_for`.

**Tech Stack:** PowerShell 5.1 (Windows) for image generation via `System.Drawing.GDI+`. Rust for binary-bundling and tray runtime.

**Spec:** [`docs/superpowers/specs/2026-05-22-theme-icons-design.md`](../specs/2026-05-22-theme-icons-design.md)

---

## Task 1: Extend `New-SkIcon` with optional ghost-text parameters

**Files:**
- Modify: `app/src-tauri/icons/generate-sk.ps1` (the `New-SkIcon` function, lines 12–87 in current HEAD)

**Why:** The riso icon needs a misregistration ghost — the same "SK" text drawn once in a ghost color offset by a few pixels, then again in the foreground color on top. The other three new families (wabi-sabi, constructivist, atomic) don't need this, but extending the single function is cleaner than forking a `New-SkRisoIcon` variant.

**Step 1: Add the optional parameters to the function signature**

Replace the `param(...)` block at lines 13–20:

```powershell
function New-SkIcon {
    param(
        [string]$OutPath,
        [string]$FontName,
        [System.Drawing.Color]$BgFrom,
        [System.Drawing.Color]$BgTo,
        [System.Drawing.Color]$TextColor,
        [bool]$Rounded,
        [System.Drawing.Color]$GhostColor = [System.Drawing.Color]::Empty,
        [int]$GhostOffsetX = 0,
        [int]$GhostOffsetY = 0,
        [System.Drawing.Color]$BorderColor = [System.Drawing.Color]::Empty,
        [single]$TextRotation = 0
    )
```

Three new optional parameter groups:

- `GhostColor` + `GhostOffsetX` + `GhostOffsetY` — for the riso misregistration ghost.
- `BorderColor` — for the atomic walnut border (the existing hard-edge path draws a border in `TextColor`, which is wrong when the family's border color differs from the SK color).
- `TextRotation` — degrees of rotation, for the constructivist -4° tilt.

**Step 2: Use `BorderColor` if set, else fall back to `TextColor`**

In the `else` branch (hard-edge square) at lines 48–54, replace:

```powershell
    } else {
        $bgBrush = New-Object System.Drawing.SolidBrush $BgFrom
        $g.FillRectangle($bgBrush, $rect)
        $bgBrush.Dispose()
        $borderPen = New-Object System.Drawing.Pen $TextColor, 8
        $g.DrawRectangle($borderPen, $rect)
        $borderPen.Dispose()
    }
```

with:

```powershell
    } else {
        $bgBrush = New-Object System.Drawing.SolidBrush $BgFrom
        $g.FillRectangle($bgBrush, $rect)
        $bgBrush.Dispose()
        $effectiveBorder = if ($BorderColor -ne [System.Drawing.Color]::Empty) { $BorderColor } else { $TextColor }
        $borderPen = New-Object System.Drawing.Pen $effectiveBorder, 8
        $g.DrawRectangle($borderPen, $rect)
        $borderPen.Dispose()
    }
```

Also extend the rounded branch (lines 32–47) to draw an optional border. After the `$g.FillPath($gradBrush, $gp)` line, add (before disposing `$gp`):

```powershell
        if ($BorderColor -ne [System.Drawing.Color]::Empty) {
            $borderPen = New-Object System.Drawing.Pen $BorderColor, 8
            $g.DrawPath($borderPen, $gp)
            $borderPen.Dispose()
        }
```

**Step 3: Draw the ghost layer + apply rotation**

Replace the text-drawing block at lines 72–77:

```powershell
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center

    $textRect = New-Object System.Drawing.RectangleF $rect.X, ($rect.Y - 8), $rect.Width, $rect.Height
    $g.DrawString('SK', $font, $textBrush, $textRect, $format)
```

with:

```powershell
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center

    # Apply rotation around the icon center if requested.
    if ($TextRotation -ne 0) {
        $cx = $rect.X + $rect.Width / 2
        $cy = $rect.Y + $rect.Height / 2
        $g.TranslateTransform($cx, $cy)
        $g.RotateTransform($TextRotation)
        $g.TranslateTransform(-$cx, -$cy)
    }

    # Ghost pass — draw "SK" once in the offset/ghost color underneath.
    if ($GhostColor -ne [System.Drawing.Color]::Empty) {
        $ghostBrush = New-Object System.Drawing.SolidBrush $GhostColor
        $ghostRect = New-Object System.Drawing.RectangleF ($rect.X + $GhostOffsetX), ($rect.Y - 8 + $GhostOffsetY), $rect.Width, $rect.Height
        $g.DrawString('SK', $font, $ghostBrush, $ghostRect, $format)
        $ghostBrush.Dispose()
    }

    # Foreground pass — primary text color on top.
    $textRect = New-Object System.Drawing.RectangleF $rect.X, ($rect.Y - 8), $rect.Width, $rect.Height
    $g.DrawString('SK', $font, $textBrush, $textRect, $format)
```

**Step 4: Run the script and verify the existing 4 icons still regenerate cleanly**

From the repo root:

```bash
pwsh app/src-tauri/icons/generate-sk.ps1
```

Expected output (in any order):

```
wrote .../sk-holo.png
wrote .../sk-memphis.png
wrote .../sk-robotic.png
wrote .../sk-corporate.png
done.
```

The existing 4 PNGs should be identical (or visually indistinguishable) from before since none of them pass `GhostColor`, `BorderColor`, or `TextRotation`. Visually open `sk-holo.png` etc. and confirm they look like they did before.

**Step 5: Verify pre-existing icons committed bytes are unchanged**

```bash
git diff --stat app/src-tauri/icons/sk-*.png
```

Expected: no output (no changes), since the new params default to no-ops.

If `sk-*.png` shows as modified, the function refactor accidentally changed rendering for the existing icons. Revert and inspect.

**Step 6: Commit**

```bash
git add app/src-tauri/icons/generate-sk.ps1
git commit -m "chore(icons): extend New-SkIcon with ghost/border/rotation params"
```

---

## Task 2: Add the `New-SkIcon` invocations for the 4 new families

**Files:**
- Modify: `app/src-tauri/icons/generate-sk.ps1` (append after line ~123, the existing 4 invocations)

**Step 1: Append the 4 new family invocations**

After the existing Corporate Overlord invocation (after the `-Rounded $false` line that ends at ~line 123 of pre-Task-1 HEAD; line numbers shift slightly post-Task-1), add:

```powershell
# Wabi-Sabi: vermillion hanko-red square (rounded), washi cream "SK" italic serif
New-SkIcon `
    -OutPath (Join-Path $here 'sk-wabi-sabi.png') `
    -FontName 'Cormorant Garamond' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 197, 74, 58)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 197, 74, 58)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 245, 239, 224)) `
    -Rounded $true

# Risograph: cream paper, federal-blue border, pink "SK" with blue misregistration ghost
New-SkIcon `
    -OutPath (Join-Path $here 'sk-riso.png') `
    -FontName 'Bebas Neue' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 245, 240, 229)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 245, 240, 229)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 255, 85, 137)) `
    -Rounded $false `
    -BorderColor ([System.Drawing.Color]::FromArgb(255, 31, 69, 135)) `
    -GhostColor ([System.Drawing.Color]::FromArgb(255, 31, 69, 135)) `
    -GhostOffsetX 18 `
    -GhostOffsetY 14

# Constructivist: agitprop-red square, wheat-cream slab "SK", tilted -4 deg
New-SkIcon `
    -OutPath (Join-Path $here 'sk-constructivist.png') `
    -FontName 'Bahnschrift' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 217, 26, 40)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 217, 26, 40)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 239, 230, 207)) `
    -Rounded $false `
    -TextRotation -4

# Mid-Century Atomic: butter cream (rounded) with walnut border, brick-orange "SK" in display serif
New-SkIcon `
    -OutPath (Join-Path $here 'sk-atomic.png') `
    -FontName 'Cooper Black' `
    -BgFrom ([System.Drawing.Color]::FromArgb(255, 245, 235, 216)) `
    -BgTo ([System.Drawing.Color]::FromArgb(255, 245, 235, 216)) `
    -TextColor ([System.Drawing.Color]::FromArgb(255, 197, 85, 42)) `
    -Rounded $true `
    -BorderColor ([System.Drawing.Color]::FromArgb(255, 45, 37, 33))
```

**Sizing notes:**
- The `GhostOffsetX 18` / `GhostOffsetY 14` for riso scales the brainstorm preview's 4 px / 3 px (designed for 128 px) up to the 256 px native size (multiplier ~4.5×).
- All hex values are the exact ones approved in the brainstorm preview, converted to ARGB tuples: e.g. `#c54a3a` → `(255, 197, 74, 58)`.
- Font choice is best-available on Windows: `Cormorant Garamond` (Google Fonts, may not be installed system-wide on Windows — the function already has a fallback to `Segoe UI Black` if not present). `Bebas Neue` likewise falls back. `Cooper Black` ships with Windows. `Bahnschrift` ships with Windows 10+.

**Step 2: Generate**

```bash
pwsh app/src-tauri/icons/generate-sk.ps1
```

Expected output:

```
wrote .../sk-holo.png
wrote .../sk-memphis.png
wrote .../sk-robotic.png
wrote .../sk-corporate.png
wrote .../sk-wabi-sabi.png
wrote .../sk-riso.png
wrote .../sk-constructivist.png
wrote .../sk-atomic.png
done.
```

**Step 3: Eyeball the four new files**

Open in any image viewer:

```bash
ls -la app/src-tauri/icons/sk-*.png
```

The 4 new files should exist at ~3–8 KB each (similar to the existing ones). Open them and confirm:

- `sk-wabi-sabi.png` — rounded square, deep red background, cream "SK" letters
- `sk-riso.png` — hard square, cream paper, blue border, pink "SK" with a visible blue ghost shifted down-and-right behind it
- `sk-constructivist.png` — hard red square with cream "SK" tilted slightly left
- `sk-atomic.png` — rounded square, cream interior, dark brown border, orange "SK"

If a font was substituted (e.g. Cormorant Garamond not installed → fallback to Segoe UI Black), the letterforms will be wrong but the colors and shape will be right. Note which fonts substituted; in that case install the missing fonts or pick a different default — but for a first pass the substitution is acceptable since the dominant identifying signal is color, not letterform.

**Step 4: Commit**

```bash
git add app/src-tauri/icons/generate-sk.ps1 app/src-tauri/icons/sk-wabi-sabi.png app/src-tauri/icons/sk-riso.png app/src-tauri/icons/sk-constructivist.png app/src-tauri/icons/sk-atomic.png
git commit -m "feat(icons): generate sk-wabi-sabi, sk-riso, sk-constructivist, sk-atomic"
```

---

## Task 3: Bundle the new PNGs into Rust + extend `tray_icon_for`

**Files:**
- Modify: `app/src-tauri/src/main.rs` (lines 12–29, the icon byte constants + `tray_icon_for` function)

**Step 1: Add 4 `include_bytes!` constants**

Find the existing block at lines 12–15:

```rust
const TRAY_HOLO_PNG: &[u8] = include_bytes!("../icons/sk-holo.png");
const TRAY_MEMPHIS_PNG: &[u8] = include_bytes!("../icons/sk-memphis.png");
const TRAY_ROBOTIC_PNG: &[u8] = include_bytes!("../icons/sk-robotic.png");
const TRAY_CORPORATE_PNG: &[u8] = include_bytes!("../icons/sk-corporate.png");
```

Add the 4 new constants directly below:

```rust
const TRAY_HOLO_PNG: &[u8] = include_bytes!("../icons/sk-holo.png");
const TRAY_MEMPHIS_PNG: &[u8] = include_bytes!("../icons/sk-memphis.png");
const TRAY_ROBOTIC_PNG: &[u8] = include_bytes!("../icons/sk-robotic.png");
const TRAY_CORPORATE_PNG: &[u8] = include_bytes!("../icons/sk-corporate.png");
const TRAY_WABI_SABI_PNG: &[u8] = include_bytes!("../icons/sk-wabi-sabi.png");
const TRAY_RISO_PNG: &[u8] = include_bytes!("../icons/sk-riso.png");
const TRAY_CONSTRUCTIVIST_PNG: &[u8] = include_bytes!("../icons/sk-constructivist.png");
const TRAY_ATOMIC_PNG: &[u8] = include_bytes!("../icons/sk-atomic.png");
```

**Step 2: Extend the `tray_icon_for` match block**

Find the existing match at lines ~18–23:

```rust
fn tray_icon_for(family: &str) -> Image<'static> {
    let bytes = match family {
        "memphis" => TRAY_MEMPHIS_PNG,
        "robotic" => TRAY_ROBOTIC_PNG,
        "corporate" => TRAY_CORPORATE_PNG,
        _ => TRAY_HOLO_PNG,
    };
```

Replace the `match` with the 7-arm version (still keeping `holo` as the default since the type signature uses `&str` rather than the typed `ThemeFamily` enum):

```rust
fn tray_icon_for(family: &str) -> Image<'static> {
    let bytes = match family {
        "memphis" => TRAY_MEMPHIS_PNG,
        "robotic" => TRAY_ROBOTIC_PNG,
        "corporate" => TRAY_CORPORATE_PNG,
        "wabi-sabi" => TRAY_WABI_SABI_PNG,
        "riso" => TRAY_RISO_PNG,
        "constructivist" => TRAY_CONSTRUCTIVIST_PNG,
        "atomic" => TRAY_ATOMIC_PNG,
        _ => TRAY_HOLO_PNG,
    };
```

**Step 3: Build and confirm the binary compiles**

```bash
cargo build -p snapper-keeper-app
```

Expected: builds successfully. If a PNG path is wrong, `include_bytes!` fails at compile time with a clear `error: couldn't read "..."`.

**Step 4: Commit**

```bash
git add app/src-tauri/src/main.rs
git commit -m "feat(tray): wire up the 4 new family icons in tray_icon_for"
```

---

## Task 4: Update the design doc commit reference + smoke

**Files:**
- (none modified)
- Manual: launch the app and verify

**Step 1: Launch the app**

From the repo root, in an interactive desktop session (per `CLAUDE.md`, this can't be done from SSH):

```bash
pnpm tauri dev
```

Wait for the library window to appear.

**Step 2: Cycle through every theme in Settings**

Open Settings → Appearance and click each of the 16 themes. For each, look at:

- The system tray icon (Windows notification area / macOS menu bar)
- The window title bar icon (Windows) / dock icon (macOS)
- The taskbar entry

The 4 existing families (`holo`, `memphis`, `robotic`, `corporate`) should look unchanged from before. The 4 new families should each show their dedicated icon:

- `wabi-sabi-*` → red rounded square with cream SK
- `riso-*` → cream square with blue border, pink SK with blue ghost
- `constructivist-*` → red square with cream SK tilted
- `atomic-*` → cream rounded square with brown border, orange SK

**Step 3: Note any font substitution issues**

If a family's "SK" looks wrong (wrong letterforms), the system was missing the named font and fell back to `Segoe UI Black` (rounded families) or `Impact` (square families). Common culprits:

- `Cormorant Garamond` — install from Google Fonts if you want the italic serif look on wabi-sabi
- `Bebas Neue` — install for the condensed riso display
- `Cooper Black` — ships with Windows but may be absent on macOS / Linux

Per the spec, font fallback is acceptable for v1 — the color + shape are the dominant signal at tray size.

**Step 4: Done. No commit needed.**

---

## Self-Review

**Spec coverage:**

- "Each icon is a 256×256 PNG generated by `generate-sk.ps1`" → Task 2 generates all 4 at 256 px (the function default).
- "Per-family parameters" table (4 rows) → Task 2 has one `New-SkIcon` invocation per row, matching the parameters.
- "Riso misregistration via optional `-GhostColor` / `-GhostOffset`" → Task 1 adds those parameters; Task 2 uses them.
- "Tilt -4°" → Task 1 adds `-TextRotation`, Task 2 uses it on constructivist.
- "Atomic walnut border" → Task 1 adds `-BorderColor` (since the existing `borderPen $TextColor` couldn't express a separate border vs text color), Task 2 uses it.
- "Rust side: 4 more `include_bytes!` + match arms" → Task 3.
- "Testing: visual via the script + cargo build + manual smoke" → Task 2 step 3 + Task 3 step 3 + Task 4.

All spec requirements have tasks. ✓

**Placeholder scan:** No TBDs / "add validation" / "similar to Task N." Code blocks are complete per step. ✓

**Task decomposition:** Parameter names (`GhostColor`, `GhostOffsetX`, `GhostOffsetY`, `BorderColor`, `TextRotation`) are consistent between Task 1 (defines them) and Task 2 (uses them). PNG filenames match: `sk-wabi-sabi.png`, `sk-riso.png`, `sk-constructivist.png`, `sk-atomic.png` in Task 2 + Task 3. Rust constant names match: `TRAY_WABI_SABI_PNG`, `TRAY_RISO_PNG`, etc. ✓

**Buildability:** A fresh engineer can copy each code block in turn. The PowerShell function refactor (Task 1) has line numbers as anchors. The Rust extension (Task 3) shows the full new block, not just the diff. Task 4 is manual but explicit. ✓
