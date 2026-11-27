# apps/desktop

Purpose: the Library desktop app (Electron, React, TypeScript). Written directly, not a fork of T3 Code (ADR 0010).
Spec: README: Desktop app and local node, Interface and screens. Design: the brand kit rules; tokens are copied into `assets/brand/`.

## Rules
- The renderer never holds a token, never opens a network connection and never reads files. Everything goes through the preload bridge to the local node.
- The bridge allowlist is generated from the Rust method table (`src/generated/app-methods.ts`); never edit generated files, run `just types`.
- Components use token variables only: no raw colours, font families or radii (`tests/shared/design-rules.test.ts` enforces it).
- Keep `contextIsolation` and `sandbox` on and `nodeIntegration` off. No `webview`, no remote content, no new permissions.
- Shortcuts use Ctrl on Windows and Linux. Every control has an accessible name and a visible focus ring.
- Every screen handles loading, empty, error, offline, sealed and no-permission states.

## Commands
- `pnpm typecheck`, `pnpm lint`, `pnpm test` (unit, no display needed)
- `cargo build -p library-node && pnpm build && pnpm test:e2e` (needs a display: WSLg, a desktop, or xvfb)
- `pnpm package:dir` builds the unpacked Linux package; `pnpm exec vitest run --config vitest.e2e.config.ts tests/e2e/packaged.test.ts` smoke-tests it
- `pnpm package:linux` builds the `.deb` and AppImage; `pnpm package:win` builds the `.appx` (on Windows)

## Notes
- Install scripts are blocked except for the packages listed in `pnpm-workspace.yaml` (`allowBuilds`).
- Packaged builds turn off Node features with Electron fuses; Playwright's launcher cannot start them, so `packaged.test.ts` attaches over CDP.
