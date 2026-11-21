# Corvid

Corvid is a Windows desktop companion and Lua addon for World of Warcraft: Forever. We’re building a combat log reader, a damage and healing meter, and a map that shows positions from combat events. The goal is to keep recent fights locally so players can review them after combat.

The desktop app shows everything on one screen: a map with your trail and numbered fights, a meter with your party's damage above its healing and a per-spell breakdown, and a list of raw events. The Rust backend follows the newest combat log as WoW writes it and sends events to the UI through the contract in `desktop/src/data/event.ts`. You can also replay any earlier log from the header, even while you play. In a browser, the UI replays a recorded sample session instead, so you can work on it without Rust or the game. The addon attempts to enable advanced combat logging on world entry, and `/corvid` checks which player and party positions the game returns.

The map draws the zone under your trail. The app draws each zone's map from the game's own files in your World of Warcraft folder, so no map art ships with Corvid. A zone the game has no map for is drawn on a grid.

| Folder | Contents |
| --- | --- |
| `addon/` | `Corvid.toc` and `Corvid.lua` |
| `desktop/src/` | React + TypeScript UI, built with Vite |
| `desktop/src/components/` | Header with the log picker and replay controls, the World of Warcraft folder prompt, and the zone map |
| `desktop/src/data/` | Event contract, event store, live and replay feed, the World of Warcraft folder, zone names, and the fight and map calculations |
| `desktop/src/views/` | Meter, map, and events views |
| `desktop/tests/` | Vitest tests |
| `desktop/src-tauri/` | Rust backend and Tauri configuration |
| `desktop/src-tauri/src/` | Log follower, combat log parser, live feed, settings, and the commands the UI calls. `wow.rs` knows where things are in the World of Warcraft folder. `casc.rs`, `db2.rs`, and `blp.rs` read the game's storage, data tables, and textures, and `maps.rs` draws and caches the maps |
| `desktop/src-tauri/tests/` | Rust tests |
| `fixtures/events/` | Sample session for the browser replay and the tests, built locally and not committed |
| `WTF/` | Your own combat logs in `CombatLog/`, kept locally and not committed. `Zones/` has zone maps exported with wow.export that were used for testing. The app doesn't use them, and builds don't include them |
| `scripts/` | Version check, and the script that builds the sample session |
| `.github/workflows/` | CI, which also builds the Windows installers on `main` |

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

The first time the app starts, it asks for your World of Warcraft folder, the one with `_forever_` in it. Picking `_forever_` or its `Logs` folder works too. The app reads combat logs from `_forever_\Logs` and draws maps from the game's files. It remembers the folder in `%APPDATA%\io.github.murder-of-crowz.corvid\settings.json`. To change it, click the folder name at the right of the header. Versions before 0.2.0 saved the log folder instead, so after updating, the app asks once.

### Live

The dropdown in the header starts on **Live**. The app follows the newest `WoWCombatLog-*.txt` in `_forever_\Logs` from its end, so fights from before the app started don't appear. Pick that log in the dropdown to see them. When the game starts a new log, the app reads the rest of the old one, then follows the new one from its start.

### Replay a log

The dropdown lists every combat log in `_forever_\Logs`, newest first, by the date and time in its name, such as `11/13/26 14:40`. It looks for new logs each time you open it. Picking one loads the whole log and plays it from the start:

- **Play** and **Pause**.
- **Prev** and **Next** jump to the start of the previous or next encounter. **Prev** on the first one goes back to the start of the log.
- **1x**, **4x**, and **20x** set the speed. Quiet stretches between fights are cut to 5 seconds.
- **Encounter N / M** shows the current encounter, followed by the log time reached.

Events keep their real times from the log. Live events keep arriving in the background during a replay, so you can review an old log while you play. Choosing **Live** again shows everything, including fights that finished in the meantime. The view never jumps to the latest on its own.

### The meter

The meter shows the latest fight, or one you pick from its dropdown. It lists you and everyone in your party or raid who did damage or healing in that fight. Damage fills the top half and healing the bottom half, each sorted from most to least and each scrolling on its own. Click a player for their per-spell breakdown. Each half opens its own, and all of them start closed. A pet's damage and healing count toward its owner, with the pet's name after each of its spells. Totals leave out overkill and overheal.

The meter only counts what's in your log. In the open world, the game may leave out what happens far from you, so a party member who wanders off can come up short.

### The map

Numbered circles mark where each fight started, counting from 1 for the oldest. The latest is filled in, and hovering one shows who the fight was against. The log only has positions during combat, so dashed lines join the trail across the stretches between fights. **Close-up** fits the view to the trail and **Whole zone** shows the full zone.

Behind the trail is the zone's map, fully explored. A zone the game has no map for is drawn on a grid. Positions from the log land on the right spots on the map with no adjustment.

#### Where the maps come from

The map art belongs to Blizzard, so Corvid never ships it. Instead, the first time a zone is shown, the app reads its map from the game's own storage in your World of Warcraft folder and draws it, fully explored. It only reads the game's files and never changes them. Drawing a zone takes about 0.3 seconds in a release build and 2 seconds under `npm run tauri dev`.

Drawn maps are kept in Corvid's addon folder, in `_forever_\Interface\AddOns\Corvid\Maps\<game version>\`, so each zone is only drawn once. The game only loads the files `Corvid.toc` lists, so it ignores them. The app makes the folder even if the addon isn't installed. A game patch starts a new folder and removes the old one. If a map can't be saved, it still shows and is drawn again next time.

Versions before 0.2.1 kept the maps in `%LOCALAPPDATA%\io.github.murder-of-crowz.corvid\maps\`. The app deletes that folder when it starts.

The log names maps by their uiMapID, such as `1420` on `MAP_CHANGE` lines. Every map the game has gets a picture, but a zone missing from `MAPS` in `desktop/src/data/zones.ts` is titled **Map 1420**. To name one, add a line like `1420: { name: "Tirisfal Glades" }`.

The PNGs in `WTF/Zones/` were exported with [wow.export](https://github.com/Kruithne/wow.export) and used for testing. The app doesn't use them, and builds don't include them. CI fails a build that has any in it. Don't commit or ship any other map art.

### How fights are read from the log

- Only lines that involve your side become events. Your side is you, your party or raid, and their pets. Fights between other players and mobs aren't tracked.
- A fight starts with damage between your side and an outsider, such as a mob, and is named after that outsider. Any other enemy that trades damage with your side before it ends joins the same fight. It ends after 6 seconds with no damage.
- Heals join the fight in progress but never start one. Damage within your own side, like Scorned by the Light hurting you, is kept as an event but belongs to no fight, so the meter doesn't count it.
- Melee is logged twice, as `SWING_DAMAGE` and `SWING_DAMAGE_LANDED`, and counts once. Damage shields like Thorns are logged as `DAMAGE_SHIELD` and count as damage.
- Your position is taken from your own events, at most once a second. It needs the zone's `MAP_CHANGE` line, which the app reads from earlier in the log when it starts partway through one.

## Work on the UI in a browser

The UI runs without Rust or the game. In a browser, it replays a sample session built from one of your own combat logs, so build that first.

1. Find a combat log in the WoW: Forever client's `Logs\` folder, for example `WoWCombatLog-110526_231729.txt`. Use one recorded with advanced combat logging on, so it has positions for the map. You can pass its full path to the script, or copy it into the repo's `WTF/CombatLog/` folder. Combat log files are gitignored wherever they are.
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

Open <http://localhost:1420>. Outside Tauri, the app replays `fixtures/events/sample-session.json` with the same controls as a replay in the app. The header shows **Sample session** in place of the log dropdown, since a browser can't read the log folder. Maps are drawn on a grid, since there are no game files to draw them from.

If the header shows **No encounters** and the views are empty, the sample session is missing. The developer console also logs `[corvid] No sample session`. Run step 2, then restart `npm run dev`.

## Event contract

`desktop/src/data/event.ts` defines the events Rust sends to the UI. Rust emits batches (`CorvidEvent[]`) on the Tauri event `corvid://events`, with camelCase field names. On the Rust side, use `#[serde(tag = "type", rename_all = "camelCase")]`. The UI drops events that don't match the contract and logs a `[corvid]` warning in the developer console, so check the console first if events don't appear. Under `npm run tauri dev`, right-click in the window and choose **Inspect** to open the console.

The UI also calls four Tauri commands. `wow_dir` and `choose_wow_dir` get and set the World of Warcraft folder. `list_logs` returns the names of the combat logs in `_forever_\Logs` for the dropdown, and `read_log` returns one log's events as a single `CorvidEvent[]` for a replay. `read_log` only accepts names that `list_logs` returns, so the UI can't read files outside that folder.

Map pictures come from the app's `map` protocol. The UI shows `http://map.localhost/<uiMapID>.png`, made with `convertFileSrc`, and the app answers with the drawn PNG, or a 404 when the game has no such map or no folder has been chosen.

## Sample session

`scripts/make-sample-session.mjs` builds the sample session from a real combat log. It keeps only events involving the player who recorded the log and the mobs they fought, and drops everyone else. Unlike the app, it drops party members' fights too, so the browser's meter shows little beyond your own numbers. The recording player's name and GUID are kept.

The script writes `fixtures/events/sample-session.json` unless you pass a different output path as a second argument. It also shortens idle stretches so the replay doesn't sit still between fights. Each developer builds their own, and `fixtures/events/` is gitignored, so the file is never committed.

Only the dev server has the sample session. Builds leave it out, so it never ends up in an installer.

Don't commit raw combat logs either. They contain other players' names. `WoWCombatLog*.txt` files are gitignored wherever they are. Outside `desktop/`, the dev server only serves `fixtures/`, so a page can't fetch your logs in `WTF/CombatLog/`.

## Tests

From `desktop/`:

```powershell
npm test
```

This reruns the tests each time you save a file. To run them once, use `npx vitest run`. Tests live in `desktop/tests/` as `*.test.ts` files and cover the event contract, text and number formatting, log names in the dropdown, fight totals, map trails and encounter markers, zone names and map pictures, and the replay controls. They need neither the game nor Rust.

Most tests use small made-up events. Four also run on your sample session, if you've built one, and check things that hold for any log, such as meter totals matching the raw events. Without a sample session, as in CI, those four are skipped.

The Rust tests live in `desktop/src-tauri/tests/`. They cover the event JSON against the contract, the parser on lines shaped like real logs, following and listing logs on real files in a temporary folder, saving the settings, finding the World of Warcraft folder from whichever folder is picked, and decoding the textures map tiles are stored as. Tauri embeds `desktop/dist` when it compiles, so build the frontend once first if `dist/` doesn't exist yet. From `desktop/`:

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

The test that draws a map needs the game installed, so it's skipped unless you ask for it, and CI never runs it. To run it on a computer with the game, from `desktop/src-tauri/`:

```powershell
$env:CORVID_WOW_DIR = "C:\path\to\World of Warcraft"
cargo test --test maps -- --ignored
```

It draws Tirisfal Glades, checks its size and that it's cached, and checks that a map the game doesn't have is remembered as missing.

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

The first command runs the tests once. The second checks TypeScript, including the tests, and builds the Vite frontend into `dist/`. The Cargo command checks the Rust backend and fails if Cargo would need to change the lockfile. The last command builds the Windows debug executable without packaging an installer. To build the release installers, use `npm run tauri build`. They land in `src-tauri/target/release/bundle/`, as `nsis/Corvid_<version>_x64-setup.exe` and `msi/Corvid_<version>_x64_en-US.msi`. The version comes from `src-tauri/tauri.conf.json`, the only place it's set, so bump it there for a release. Builds include neither map art nor the sample session. The installers aren't signed yet, so Windows SmartScreen warns before running them.

## CI

`.github/workflows/ci.yml` runs on every push on any branches, and when started manually from the **Actions** tab. It has three jobs:

- **Web UI** (Ubuntu): installs npm packages, runs the tests, then type-checks and builds the frontend. The build step runs even when tests fail, so one run reports both.
- **Rust backend** (Windows): installs the toolchain pinned in `rust-toolchain.toml`, runs `npm run check-version`, builds the frontend (Tauri embeds `desktop/dist` when it compiles), then runs `cargo test --locked`.
- **Windows installers** (Windows): runs only on `main`, such as after a pull request is merged, and only once both jobs above pass. It runs `npm run tauri build`, fails if map art or a sample session got into the build, and uploads the NSIS and MSI installers as an artifact named `Corvid-<version>-<commit>`. Download it from the run's **Summary** page, under **Artifacts**.

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
