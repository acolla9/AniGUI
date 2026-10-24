# AniGUI search integration status

## Wired in

- The `ani_engine.rs` module is part of the Tauri Rust backend.
- Rust dependencies for HTTP requests, HTML parsing, base64 decoding, regexes, and JSON are in `src-tauri/Cargo.toml`.
- `search_anime` is an async Tauri command and calls `AniEngine::search`.
- The React search form calls the command, displays returned titles, shows result counts, and surfaces backend errors.
- Search result cards deliberately do not pretend to play episodes yet.

## Not verified here

- This environment does not have Cargo installed, so the Rust backend could not be compiled.
- `npm install` did not complete within the available command time, so the TypeScript/Vite build could not be run either.
- Live requests to the provider were not tested. The provider may block automated requests or change its markup/endpoints.

## Next local test

From the project directory, run:

```powershell
npm install
npm run tauri dev
```

If Rust compilation fails, run `cargo check` from `src-tauri` and use the first compiler error to guide fixes. Then try searching for a common anime title. A successful frontend build alone does not prove the provider parser is still current.

## Remaining milestones

1. Add episode list command and UI.
2. Add stream resolution and select a playback strategy.
3. Package a player and review its redistribution terms.
4. Add history, settings, installer testing, and robust provider failure handling.

The upstream `ani-cli` license is GPL-3.0-or-later. Preserve notices and meet the license requirements for distributed derivative work.


Search-results fix: search parsing is now scoped to `.film_list-search .film-detail` so sidebar/recommendation cards are not mixed into results. Result IDs are de-duplicated across the full result list, not just adjacent entries. This parser change has not been live-tested against the provider markup.


## Search parser follow-up

The `.film_list-search` selector proved too restrictive in the previous patch and could return zero results. Search parsing now follows the original ani-cli strategy: truncate the raw HTML before `id=\"main-sidebar\"`, then parse `.film-detail` entries and globally deduplicate IDs. This still requires testing against a live provider response; the provider site was not reachable from the build environment.


## Playback integration

Added `src-tauri/src/player.rs` (player discovery, argument building, spawning; unit-tested) and the `play_episode` / `detect_player` commands in `lib.rs`. The episode cards in `App.tsx` now start playback, with Sub/Dub and quality controls and player status.

Verified in the build environment: `player.rs` unit tests pass, including one that launches a fake `mpv` and checks the exact arguments and exit-code handling; `App.tsx` type-checks against stubbed library types. **Not verified:** the full Tauri build (no WebKit libraries here, and crates.io/npm were unreachable) and live playback against the provider. Run `npm install && npm run tauri dev`, and if `cargo` reports an error in `lib.rs`, it is the first place to look.
