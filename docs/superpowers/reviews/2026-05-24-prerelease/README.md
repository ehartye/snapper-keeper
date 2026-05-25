# Pre-release multi-perspective review — 2026-05-24

Multi-perspective review of the snapper-keeper codebase before cutting v1.0. Run via `h-superpowers:perspective-review` (team-driven).

## Perspectives

- **Adversary** — security/threat surface, signing chain, IPC trust, data exposure
- **Operator** — release pipeline, auto-updater failure modes, diagnostics, "3am on fire" scenarios
- **Maintainer** — coupling, architectural drift, onboarding, implicit knowledge
- **Testing Strategy** — coverage, fragility, untested perimeters

## Layout

- [`synthesis.md`](synthesis.md) — consolidated report with prioritized backlog
- [`round-1/`](round-1/) — independent findings (locked)
- [`round-2/`](round-2/) — cross-pollination (Reactions, Tensions, New Insights)

## Headline

Hold v1.0 against the GitHub Releases endpoint. Six blockers flagged by 2+ perspectives:

1. Stored XSS via FTS snippet + `csp: null` + full-IPC capability
2. Clipboard watcher captures secrets, no exclusions, dead `sensitive` flag
3. PRIVACY.md / spec / README make material claims the binary cannot keep
4. Uniform capability grants every window full IPC blast radius
5. No file-based logging; stdout discarded on packaged Windows
6. Pre-migration backup promised but absent; `recoverable` flag is a lie

See `synthesis.md` § Executive Summary for the full set.
