# AniGUI

A Windows desktop app for finding and watching anime, built on the ideas of [ani-cli](https://github.com/pystardust/ani-cli). Search a title, pick an episode, and it opens in mpv (bundled) or VLC. No terminal, Bash or WSL needed.

Built with [Tauri 2](https://tauri.app), React, TypeScript and Rust.

> **Status: early testing releases (0.1.x).** It works, but it depends on third-party streaming providers that can change or block requests at any time. See [Troubleshooting](#troubleshooting).

## Features

- Search for anime and browse its episode list
- **Sub / Dub** switch per title, with a default you can set
- Quality selection (best, 1080p, 720p, 480p, 360p, worst), falling back to the best available if your pick isn't offered
- Plays through **mpv** (bundled with the installer) or **VLC**, with the right headers, separate audio track and subtitles handled for you
- **Autoplay next episode** (mpv)
- Watch history with "Recently watched" on the home page; stored on your computer only
- Accent color themes

## Install

1. Download the latest `AniGUI_x.y.z_x64-setup.exe` from the [Releases page](https://github.com/acolla9/AniGUI/releases/latest).
2. Run it. It installs for the current user only, so no administrator rights are needed.
3. Launch **AniGUI**.

**Requirements:** Windows 10 or 11, 64-bit, and an internet connection. AniGUI uses Microsoft's WebView2 runtime, which is already part of current Windows; the installer fetches it if it's missing.

Windows SmartScreen may warn about an unrecognized app, because the installer isn't code-signed. Choose **More info → Run anyway** if you trust the download.

## Using it

1. Open **Search**, type a title, and pick a result.
2. Choose **Sub** or **Dub**, then click an episode.
3. The episode opens in your player. When it ends, the next one starts automatically (mpv) or use the **Next** button.

### Settings

- **Player:** mpv (default) or VLC. You can point to your own copy with the optional location fields.
- **Playback:** default Sub/Dub, default quality, autoplay next.
- **Watch history:** remove entries or clear everything.

Settings and history are stored as `settings.json` and `history.json` in `%APPDATA%\dev.anigui.gui`.

## Troubleshooting

**"The player could not play episode N (exit code 2)"**
mpv couldn't open the stream. The message now ends with mpv's own error lines. Common causes:

- *TLS / certificate errors:* fixed in 0.1.2 by bundling a CA certificate file. If you use your own mpv, update Windows or your mpv build.
- *Expired stream or provider change:* try again, or try another quality or Sub/Dub.
- *Network blocking:* a VPN, firewall or DNS issue can block the provider's video servers.

mpv's full log for the last attempt is saved at `%TEMP%\anigui-mpv.log`. Include it if you open an issue.

**"mpv wasn't found"**
Reinstall AniGUI (mpv is bundled), or set the location in **Settings → Player**, or switch to VLC. You can also install it yourself with `winget install shinchiro.mpv`, `scoop install mpv` or `choco install mpvio.install`.

**Search returns nothing**
The streaming provider may be down, have changed its website, or be blocking requests. This is the most likely thing to break over time; please report it with the date and what you searched for.

## Build from source

Prerequisites: [Node.js](https://nodejs.org), [Rust](https://rustup.rs), and the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (Microsoft C++ Build Tools and WebView2).

```powershell
git clone https://github.com/acolla9/AniGUI.git
cd AniGUI
npm install
npm run tauri dev       # run in development
npm run tauri build     # build the installer
```

The installer is written to `src-tauri\target\release\bundle\nsis\`.

mpv isn't stored in the repository. `npm run tauri build` runs `scripts/fetch-mpv.mjs --require`, which downloads the latest 64-bit Windows build from [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake) and Mozilla's CA bundle (via [curl.se](https://curl.se/docs/caextract.html)) into `resources/mpv/`. Tauri bundles that folder and the app finds it next to the executable. The build fails if the download fails, so a release can't ship without them.

- `npm run fetch-mpv` re-downloads the newest mpv.
- `npm run tauri dev` tries to fetch mpv too, but continues without it if you're offline.
- Set `GITHUB_TOKEN` if GitHub rate-limits the download.
- To use a different player, set `ANIGUI_PLAYER` to the path of the executable.

### Project layout

| Path | What it is |
| --- | --- |
| `src/` | React + TypeScript user interface |
| `src-tauri/src/ani_engine.rs` | Provider search, episode list and stream resolution (adapted from ani-cli) |
| `src-tauri/src/player.rs` | Finds and launches mpv or VLC |
| `src-tauri/src/persist.rs`, `store.rs` | Settings and watch history |
| `scripts/fetch-mpv.mjs` | Downloads mpv and the CA bundle at build time |

## Legal

AniGUI is free software, licensed under the **GNU General Public License v3.0 or later**. See [LICENSE](LICENSE). It contains code adapted from ani-cli (GPL-3.0-or-later) and ships mpv (GPL), so the whole distribution is covered by the GPL.

Third-party components and their licenses are listed in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). That file also explains how to obtain the source code of the bundled GPL components.

AniGUI doesn't host, store or distribute any video content. It is a client that accesses third-party websites, which can change or disappear without notice. You are responsible for making sure your use complies with the laws of your country and with the terms of those services.

## Credits

- [ani-cli](https://github.com/pystardust/ani-cli) by pystardust and contributors, the original idea and workflow
- [mpv](https://mpv.io) and [shinchiro's Windows builds](https://github.com/shinchiro/mpv-winbuild-cmake)
- [Tauri](https://tauri.app), [React](https://react.dev), [Lucide](https://lucide.dev)