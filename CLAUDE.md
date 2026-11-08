# Achaja — code repository

A hands-free voice assistant for Windows built on Claude Code: Tauri 2 (Rust) + React 19 +
TypeScript (Vite), tests on Vitest and `cargo test`. Public repository `github.com/Banzamel/AchajaVA`
(MIT). The rules Achaja's own Claude session follows are **not** this file: they ship with the
program in `resources/claude/` (CLAUDE.md, skills, the hooks template, the config template).

## Code structure

The same rules in all of the owner's apps.

1. **File size:** aim for at most 150 lines, hard limit 200. Language files, JSON and data are exempt.
2. **One responsibility per file.** One React component per file. A file is named after its main
   export: `PascalCase.tsx` for components and classes, `camelCase.ts` for modules.
3. **Folders:**
   - `src/app/` — entry (`main.tsx`: index.html is Achaja's one window; Settings are a view in it),
     service wiring, start-up, the title bar, tabs
   - `src/features/<feature>/` — `components/`, `hooks/`, services, `index.ts` public API:
     `assistant` (polling, header, avatar, controls, menu), `chat`, `terminal`, `agents`, `plan`,
     `claudeAccount`, `log`, `settings`
   - `src/shared/{ui,hooks,i18n,lib}` — reusable pieces without feature knowledge (`ui/titlebar` is
     byte-identical in all Banzamel apps — change it in all of them or not at all)
   - `src/core/<domain>/` — pure logic without React or Tauri (state of the assistant, messages,
     Claude texts, palettes, plan rows, the settings form)
   - `src/platform/` — the **only** place that imports `@tauri-apps/*`
   - `src/lang/` (pl.json, en.json), `src/styles/` (CSS per area)
4. **Classes for stateful services** (`AssistantService`, `ClaudeService`, `SettingsService`,
   `ThemeService`, `ModalService`, `I18nService`) with dependencies injected through the
   constructor; pure functions for calculations.
5. **TypeScript strict**; no `any` except wrapped at JSON/IPC boundaries.
6. Components mostly render; logic in hooks and services; state in small `Store`s owned by services.
7. Features import each other only through their `index.ts`.
8. **Rust:** `main.rs` and `lib.rs` are wiring only. All logic is in the crate `achaja-core`
   (`src-tauri/core`, no Tauri dependency) with one module folder per area: `voice/` (microphone,
   playback, outputs, `speak`), `stt/` (Vosk loaded at run time), `tts/` (Edge read-aloud with the
   Sec-MS-GEC v2 token), `cast/` (Google Cast), `console/` (keys into / screen of the hidden Claude
   console), `listener/`, `claude/` (session start, workspace, account, sessions), `hooks/`,
   `agents/`, `mail/` (+ `watch/`), `gmail`, `phone/`, `homeassistant/`, `plan/` (+ jobs),
   `settings/`, `backup`, `update`, `cli/`. The app crate (`src-tauri/src`) has `commands/`
   (thin wrappers), `host/` (listener thread, console reader, supervisor, shutdown), `window/`,
   `tray.rs`, `updater.rs`, `launch.rs`. One error type (`achaja_core::error::AppError`,
   serde-serializable), no `unwrap`/`expect` in command paths.
9. **Tests mirror the source tree:** `test/` for TypeScript (Vitest), `#[cfg(test)]` modules in the
   core for Rust (`cargo test -p achaja-core`, runs on Linux in Docker). Windows-only code sits
   behind `cfg(windows)` with a stub for other systems.
10. Code and comments in **English**; comments explain **why**. User-facing texts of the session
    tools (`Achaja.exe <command>`) stay Polish as before; the window is translated (pl/en).

## How it fits together

- `Achaja.exe` without arguments is the window (tray, listener thread, console reader,
  supervisor). `Achaja.exe <command>` is the command line of the voice session's tools and hooks
  (`plan`, `jobs`, `mail`, `gmail`, `phone`, `ha`, `agent`, `claude`, `backup`, `update`,
  `output`, `speak`, `hook`, `devices`, `audio`) — it runs before any window code (`main.rs`).
- Processes talk through small JSON files in `data\state` (the same names as the Python versions):
  hooks write `achaja_state.json`, `approval.json`, `followup.json`, `chat.jsonl`; `achaja speak`
  processes queue through `tts.pid` and stop on `tts_stop.json`; the mail watcher
  (`Achaja.exe mail watch`), phone alerts (`phone run`) and background jobs (`jobs exec`) are
  separate processes started by the window.
- The hidden Claude session runs in `data\` (its workspace: `.claude\` with the rules and the
  hooks settings written on every start, `achaja.cmd`, `.mcp.json`).

## Where the program writes

Only into its own folder: `data\` (config.json, .mcp.json, .claude\, state\, models\, webview\ =
the WebView2 profile via `data_directory`) and `backups\`. There is one window: no second webview, no
second profile. Never into `%APPDATA%`, `%LOCALAPPDATA%` or the user profile — check every
new plugin for `app_*_dir` use. Allowed outside: `%TEMP%` during install/update, the Start menu
shortcuts, the `HKCU\...\Run` entry for "start with Windows". Claude Code's own files
(`~/.claude`, `~/.claude.json`: the folder trust and the Claude tab's tidying) belong to Claude Code.
Development: `ACHAJA_DATA` (data folder), `ACHAJA_RESOURCES` (`resources/`), `ACHAJA_NO_LISTENER`
(the window without the listener and the Claude session).

## Installer

NSIS via Tauri, hooks in `src-tauri/windows/hooks.nsh` + `achaja-migrate.nsh`: default folder
`<system drive>\Banzamel\Achaja`, never inside the profile; the Python version (Inno Setup, AppId
`{6F0B7C2E-4B7A-4E0C-9C55-3A1C4E7D2B90}`) has its data copied into `data\`, is uninstalled silently
and checked gone before its leftovers are removed; Inno-style `/SILENT` / `/VERYSILENT` from its
updater become passive / silent mode. Uninstall deletes `data\` and `backups\` only when "Delete
the application data" is ticked; updates never do.

## Commands

- `npm test` — Vitest; `npm run typecheck`; `npm run build` — tsc + vite build
- `npm run fetch-vosk` — libvosk.dll and its runtime into `resources/vosk/` (git-ignored)
- `npm run dist` — the Windows installer through Docker (image `tauri-build`, `docker/build.sh`),
  into `release/`; it also runs the tests and `cargo test -p achaja-core`
- `npm start` — `tauri dev` (needs Rust + MSVC on Windows)

## Release

Version source: `package.json` (`tauri.conf.json` reads it). Put the changes under
`## [Unreleased]` in `CHANGELOG.md`, run `npm run release:minor` (or patch/major: bumps, stamps
the changelog, commits "Release vX.Y.Z" and tags), then `git push origin main --follow-tags`.
The workflow checks the tag, builds the signed installer and publishes `AchajaSetup-<version>.exe`
and `latest.json` (the signature is inside it; no `.sig`, no portable zip). The updater key is
`C:\Users\polac\.tauri\achaja.key` (never regenerate it); the repository needs the secrets
`TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD`, empty).
