# AniCLI GUI (Windows-first starter)

A Tauri 2 + React + TypeScript desktop shell for a Windows-friendly GUI inspired by `ani-cli`.

## Current status

This is the first scaffold, not a finished anime streaming app yet.

- [x] Tauri 2 project and Windows NSIS installer configuration
- [x] Dark desktop UI with navigation and search form
- [x] Rust command bridge (`get_app_status`, `search_anime`)
- [x] Connect native Rust anime search/source parsing (provider integration not live-tested)
- [x] Episode selection and stream resolution
- [x] Playback through an external player (mpv preferred, VLC fallback)
- [ ] Bundle mpv with the installer
- [ ] Installer prerequisite handling and release testing

The search command now calls the Rust engine and displays parsed results. The provider can change its markup or block requests, so live behavior must be tested from the desktop app.

## Development prerequisites

For development only, install Node.js, Rust, and the Windows build prerequisites listed in the Tauri v2 Windows setup guide (including Microsoft C++ Build Tools and WebView2). The *end-user installer* can be designed to handle the app's runtime dependencies separately.

```powershell
npm install
npm run tauri dev
```

Build the frontend:

```powershell
npm run build
```

Build the Windows NSIS installer (run on Windows with the Tauri toolchain installed):

```powershell
npm run tauri build
```

## Design/implementation notes

- The original `ani-cli` project is a shell script, so it cannot be executed natively by Windows without a compatible shell/runtime.
- The intended direction is to port/adapt the source workflow behind Rust commands so the installed app does not require users to install Bash or WSL.
- A bundled player such as mpv needs to be packaged and configured in a later milestone. Review its current redistribution terms before shipping it.
- Review and comply with the original `ani-cli` license and any source/provider terms before distributing derivative code or integrations.
- The app needs network access for search and streaming once the engine is integrated. Site changes may break scraper-based functionality.


## Search engine integration

The native Rust module is in `src-tauri/src/ani_engine.rs`. Tauri exposes it as `search_anime`, and the React UI renders real result titles or the returned error. Episode selection and playback are not connected yet. The implementation uses scraper-based parsing against the current provider endpoints, so it may need maintenance if those endpoints change.


## Playback

Clicking an episode calls the `play_episode` Rust command, which resolves the stream (`AniEngine::resolve_episode`), picks the requested quality (falling back to best, like ani-cli), and launches the player from `src-tauri/src/player.rs` with the same flags ani-cli uses: `--referrer`, `--audio-file`, `--sub-file`, `--force-media-title` for mpv, and the VLC equivalents. A `playback-ended` event tells the UI when the player closes or fails.

Player lookup order: `ANIGUI_PLAYER` (or `ANI_CLI_PLAYER`) environment variable → mpv next to the app (`mpv.exe`, `mpv/mpv.exe`, `resources/mpv/mpv.exe`) → common Windows install locations (Scoop, Chocolatey, Program Files) → `mpv` on PATH → VLC. For development, `scoop install mpv` or `winget install mpv` is enough.

An external player is used on purpose: the streams require a `Referer` header and a separate audio track, which a webview `<video>` element cannot send or mux.
