# Corvid

Corvid is a Windows desktop companion and Lua addon for World of Warcraft: Forever. We’re building a combat log reader, a damage and healing meter, and a map that shows positions from combat events. The goal is to keep recent fights locally so players can review them after combat.

The desktop app shows everything on one screen: a map with your trail and numbered fights, a damage and healing meter with a per-spell breakdown, and a list of raw events. The Rust backend follows the newest combat log as WoW writes it and sends events to the UI through the contract in `desktop/src/data/event.ts`. You can also replay any earlier log from the header, even while you play. In a browser, the UI replays a recorded sample session instead, so you can work on it without Rust or the game. The addon attempts to enable advanced combat logging on world entry, and `/corvid` checks which player and party positions the game returns.

The map draws the zone under your trail, from a map image you export from your own game files. Tirisfal Glades and Silverpine Forest are set up so far. Other zones are drawn on a grid.

| Folder | Contents |
| --- | --- |
| `addon/` | `Corvid.toc` and `Corvid.lua` |
| `desktop/src/` | React + TypeScript UI, built with Vite |
| `desktop/src/components/` | Header with the log picker and replay controls, and the log folder prompt |
| `desktop/src/data/` | Event contract, event store, live and replay feed, log folder, and the fight and map calculations |
| `desktop/src/views/` | Meter, map, and events views |
| `desktop/tests/` | Vitest tests |
| `desktop/src-tauri/` | Rust backend and Tauri configuration |
| `desktop/src-tauri/src/` | Log follower, combat log parser, live feed, settings, and the commands the UI calls |
| `desktop/src-tauri/tests/` | Rust tests |
| `fixtures/events/` | Sample session for the browser replay and the tests, built locally and not committed |
| `WTF/` | Your own combat logs in `CombatLog/` and exported zone maps in `Zones/`, kept locally and not committed |
| `scripts/` | Version check, and the script that builds the sample session |
| `.github/workflows/` | CI |

## Tech stack and pinned versions

| Component | Version |
| --- | --- |
| Node.js / npm | 26.11.1 / 11.20.0 |
| Rust / Cargo | 1.99.1, Windows x64 MSVC toolchain |
| React / React DOM | 19.3.1 / 19.3.1 |
| TypeScript / React type packages | 6.0.3 / 19.3.0 |
| Vite / React plugin | 8.3.6 / 6.2.1 |
| Vitest | 5.0.5 |
| Tauri Rust crate / Tauri build crate | 2.12.5 / 2.7.4 |
| Tauri npm CLI / JavaScript API | 2.12.4 / 2.12.4 |
| Tauri opener plugin | 2.7.2 on both sides |
| Lua addon | WoW: Forever client Lua, TOC interface 16001 |

`desktop/package.json` sets the exact Node.js, npm, and direct frontend dependency versions. `desktop/.npmrc` enforces the Node.js and npm versions when installing and saves new dependencies as exact versions. `desktop/package-lock.json` also pins transitive npm dependencies. Rustup reads `rust-toolchain.toml` for the compiler and Cargo versions. Cargo resolves Rust crates using `desktop/src-tauri/Cargo.lock`. The addon runs in WoW’s Lua runtime.

## Set up the desktop app on Windows

Install these prerequisites:

- Node.js **26.11.1** and npm **11.20.0**. Select the Node version in your version manager or install it from the [Node.js release archive](https://nodejs.org/dist/v26.11.1/). If your npm version differs, run `npm install --global npm@11.20.0`.
- Rustup to install the pinned Windows MSVC toolchain. [Rust installation](https://rust-lang.org/tools/install/)
- Microsoft C++ Build Tools with the **Desktop development with C++** workload and the Microsoft Edge WebView2 Runtime. [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)

If Rust is missing, install it through Rustup. On Windows with WinGet:

```powershell
winget install --id Rustlang.Rustup --exact --source winget
```

Open a fresh PowerShell window. Install the exact Rust toolchain and confirm the commands are available:

```powershell
rustup toolchain install 1.99.1-x86_64-pc-windows-msvc --profile minimal
node --version
npm --version
rustc --version
cargo --version
```

From the root of your Corvid checkout:

```powershell
npm --prefix desktop ci
npm run check-version
cd desktop
npm run tauri dev
```

`npm ci` installs the frontend dependencies from the checked-in lockfile. `npm run tauri dev` starts Vite and opens the desktop window. Press **Ctrl+C** in the terminal to stop it.

Run `npm run check-version` from the repo root after installing dependencies or changing your toolchain. It prints each current and expected version, checks the installed direct frontend packages against the pins, and exits with status 1 if anything is missing or mismatched. The root command requires no separate npm install. To upgrade the baseline, update the relevant pins and lockfiles together and rerun the version and build checks.

Install npm packages from `desktop/`, not the repo root. The root `package.json` only holds the `check-version` script.

## Use the desktop app

Type `/combatlog` in game to start logging. The map needs advanced combat logging, which puts positions in the log. The addon tries to turn it on for you.

The first time the app starts, it asks for the folder WoW writes combat logs to, usually `_forever_\Logs` in the WoW install. It remembers the folder in `%APPDATA%\io.github.murder-of-crowz.corvid\settings.json`. To change it, click the folder name at the right of the header.

### Live

The dropdown in the header starts on **Live**. The app follows the newest `WoWCombatLog-*.txt` in the folder from its end, so fights from before the app started don't appear. Pick that log in the dropdown to see them. When the game starts a new log, the app reads the rest of the old one, then follows the new one from its start.

### Replay a log

The dropdown lists every combat log in the folder, newest first, by the date and time in its name, such as `11/13/26 14:40`. It looks for new logs each time you open it. Picking one loads the whole log and plays it from the start:

- **Play** and **Pause**.
- **Prev** and **Next** jump to the start of the previous or next encounter. **Prev** on the first one goes back to the start of the log.
- **1x**, **4x**, and **20x** set the speed. Quiet stretches between fights are cut to 5 seconds.
- **Encounter N / M** shows the current encounter, followed by the log time reached.

Events keep their real times from the log. Live events keep arriving in the background during a replay, so you can review an old log while you play. Choosing **Live** again shows everything, including fights that finished in the meantime. The view never jumps to the latest on its own.

### The map

Numbered circles mark where each fight started, counting from 1 for the oldest. The latest is filled in, and hovering one shows who the fight was against. The log only has positions during combat, so dashed lines join the trail across the stretches between fights. **Close-up** fits the view to the trail and **Whole zone** shows the full zone.

Behind the trail is the zone's map, if you've exported its image. A zone without one is drawn on a grid. Positions from the log land on the right spots on the image with no adjustment.

#### Add a zone map

The map art belongs to Blizzard, so it isn't in the repo. Never commit it or ship it in a build. Each developer exports their own from their game install:

1. Use [wow.export](https://github.com/Kruithne/wow.export) on your WoW install to export the zone's map as a PNG. It names the file `Zone_<zoneId>_<Name>.png`, such as `Zone_85_Tirisfal_Glades.png`, at 1002×668, the size the map is drawn at.
2. Put the file in the repo's `WTF/Zones/` folder, which is gitignored.
3. The log doesn't use the zone ID in the file name. It names maps by their uiMapID instead, such as `1420` on `MAP_CHANGE` lines. If the zone isn't in `MAPS` in `desktop/src/views/MapView.tsx` yet, add a line that pairs the two, like `1420: { name: "Tirisfal Glades", zoneId: 85 }`.
4. Restart `npm run tauri dev` or `npm run dev` to pick up the new image.

A build bundles every image in `WTF/Zones/` into the app. Empty the folder before building anything you'll give to someone else.

### How fights are read from the log

- Only lines that involve you become events. Fights between party members and mobs aren't tracked.
- A fight starts with damage between you and an outsider, such as a mob, and is named after them. Any other enemy hit before it ends joins the same fight. It ends after 6 seconds with no damage.
- Heals join the fight in progress but never start one. Damage within your own side, like Scorned by the Light hurting you, is kept as an event but belongs to no fight, so the meter doesn't count it.
- Melee is logged twice, as `SWING_DAMAGE` and `SWING_DAMAGE_LANDED`, and counts once. Damage shields like Thorns are logged as `DAMAGE_SHIELD` and count as damage.
- Your position is taken from your own events, at most once a second. It needs the zone's `MAP_CHANGE` line, which the app reads from earlier in the log when it starts partway through one.

## Work on the UI in a browser

The UI runs without Rust or the game. In a browser, it replays a sample session built from one of your own combat logs, so build that first.

1. Find a combat log in the WoW: Forever client's `Logs\` folder, for example `WoWCombatLog-110526_231729.txt`. Use one recorded with advanced combat logging on, so it has positions for the map. You can pass its full path to the script, or copy it into the repo's `WTF/CombatLog/` folder, which is gitignored.
2. From the repo root, build the sample session:

   ```powershell
   node scripts/make-sample-session.mjs WTF/CombatLog/WoWCombatLog-110526_231729.txt
   ```

   It prints a summary such as `240 events, 14 fights, 219 s replay -> fixtures/events/sample-session.json`.

   > The "WTF/CombatLog/WoWCombatLog-110526_231729.txt" is just a placeholder for now, it is where I currently have logs stored at. Will update loc later on.

3. From `desktop/`, start the dev server:

   ```powershell
   npm run dev
   ```

Open <http://localhost:1420>. Outside Tauri, the app replays `fixtures/events/sample-session.json` with the same controls as a replay in the app. The header shows **Sample session** in place of the log dropdown, since a browser can't read the log folder.

If the header shows **No encounters** and the views are empty, the sample session is missing. The developer console also logs `[corvid] No sample session`. Run step 2, then restart `npm run dev`.

## Event contract

`desktop/src/data/event.ts` defines the events Rust sends to the UI. Rust emits batches (`CorvidEvent[]`) on the Tauri event `corvid://events`, with camelCase field names. On the Rust side, use `#[serde(tag = "type", rename_all = "camelCase")]`. The UI drops events that don't match the contract and logs a `[corvid]` warning in the developer console, so check the console first if events don't appear. Under `npm run tauri dev`, right-click in the window and choose **Inspect** to open the console.

The UI also calls four Tauri commands. `logs_dir` and `choose_logs_dir` get and set the log folder. `list_logs` returns the combat log names for the dropdown, and `read_log` returns one log's events as a single `CorvidEvent[]` for a replay. `read_log` only accepts names that `list_logs` returns, so the UI can't read files outside the folder.

## Sample session

`scripts/make-sample-session.mjs` builds the sample session from a real combat log. It keeps only events involving the player who recorded the log and the mobs they fought, and drops everyone else. The recording player's name and GUID are kept.

The script writes `fixtures/events/sample-session.json` unless you pass a different output path as a second argument. It also shortens idle stretches so the replay doesn't sit still between fights. Each developer builds their own, and `fixtures/events/` is gitignored, so the file is never committed.

Don't commit raw combat logs either. They contain other players' names. `WTF/` and `WoWCombatLog*.txt` are gitignored. Outside `desktop/`, the dev server only serves `fixtures/` and `WTF/Zones/`, so a page can't fetch your logs in `WTF/CombatLog/`.

## Tests

From `desktop/`:

```powershell
npm test
```

This reruns the tests each time you save a file. To run them once, use `npx vitest run`. Tests live in `desktop/tests/` as `*.test.ts` files and cover the event contract, text and number formatting, log names in the dropdown, fight totals, map trails and encounter markers, and the replay controls. They need neither the game nor Rust.

Most tests use small made-up events. Four also run on your sample session, if you've built one, and check things that hold for any log, such as meter totals matching the raw events. Without a sample session, as in CI, those four are skipped.

The Rust tests live in `desktop/src-tauri/tests/`. They cover the event JSON against the contract, the parser on lines shaped like real logs, following and listing logs on real files in a temporary folder, and saving the settings. Tauri embeds `desktop/dist` when it compiles, so build the frontend once first if `dist/` doesn't exist yet. From `desktop/`:

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Install and test the WoW addon

1. Open `Interface\AddOns\` under the active WoW: Forever client folder (for example, `_forever_\Interface\AddOns\`).
2. Copy the two files from the repo’s `addon/` directory into a folder named `Corvid` directly under `AddOns`.

   ```text
   Interface\AddOns\Corvid\
   ├── Corvid.toc
   └── Corvid.lua
   ```

3. Launch WoW, enable **Corvid** in the character selection AddOns list, then enter the world.
4. Type `/corvid` in game to check position reporting. After editing Lua, copy the updated file over and run `/reload`.
5. Fight a mob and check the active client’s `Logs\` directory for the newest `WoWCombatLog*.txt` file. Forever can use timestamped filenames. If logging is off, `/combatlog` toggles it on.

The desktop app reads the combat log itself, so it runs without the addon. Without advanced combat logging, though, the log has no positions and the map stays empty.

## Build checks

Run from `desktop/`:

```powershell
npx vitest run
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug --no-bundle
```

The first command runs the tests once. The second checks TypeScript, including the tests, and builds the Vite frontend into `dist/`. The Cargo command checks the Rust backend and fails if Cargo would need to change the lockfile. The last command builds the Windows debug executable without packaging an installer. To build a release installer, use `npm run tauri build`. Builds include any zone maps in `WTF/Zones/`, so empty that folder before sharing one.

## CI

`.github/workflows/ci.yml` runs on every push on any branches, and when started manually from the **Actions** tab. It has two jobs:

- **Web UI** (Ubuntu): installs npm packages, runs the tests, then type-checks and builds the frontend. The build step runs even when tests fail, so one run reports both.
- **Rust backend** (Windows): installs the toolchain pinned in `rust-toolchain.toml`, runs `npm run check-version`, builds the frontend (Tauri embeds `desktop/dist` when it compiles), then runs `cargo test --locked`.

## Fix: Tauri reports that Cargo was not found

We saw this when starting the app:

```text
failed to run command cargo metadata --no-deps --format-version 1: program not found
```

Installing Rustup and selecting the stable toolchain made Cargo available on disk. The existing PowerShell session still had an older PATH, so Tauri could not launch it. Fully quit and reopen the app hosting the terminal, such as VS Code or Windows Terminal, to pick up the updated user PATH.

To fix the current PowerShell session immediately, run these commands from `desktop/`:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo --version
cargo metadata --no-deps --format-version 1 --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

We verified that this PATH change lets Cargo read Corvid’s manifest. If `cargo --version` still fails, check whether Cargo was installed in the usual Rustup location:

```powershell
Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"
```

If this returns `False`, install Rust through Rustup. If Rustup reports that the pinned toolchain is missing, run the toolchain install command in the setup section. The repo now selects that exact release. If Cargo can read the manifest but compilation reports that `link.exe` is missing, check the C++ workload in Visual Studio Installer.
