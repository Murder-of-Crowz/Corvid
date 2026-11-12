# Corvid

Corvid is a Windows desktop companion and Lua addon for World of Warcraft: Forever. We’re building a combat log reader, a damage and healing meter, and a map that shows positions from combat events. The goal is to keep recent fights locally so players can review them after combat.

The desktop UI has three views: a damage and healing meter with a per-spell breakdown, a map with your trail and position, and a raw event list. It reads events from the Rust backend through the contract in `desktop/src/data/event.ts`. Until the Rust log reader sends events, you can work on the UI in a browser, where it replays a recorded session. The addon attempts to enable advanced combat logging on world entry, and `/corvid` checks which player and party positions the game returns. The next milestone is the Rust log reader, which will send new combat log events to the UI while WoW is running.

| Folder | Contents |
| --- | --- |
| `addon/` | `Corvid.toc` and `Corvid.lua` |
| `desktop/src/` | React + TypeScript UI, built with Vite |
| `desktop/src/data/` | Event contract, event store, event feed, and the fight and map calculations |
| `desktop/src/views/` | Meter, map, and events views |
| `desktop/tests/` | Vitest tests |
| `desktop/src-tauri/` | Rust backend and Tauri configuration |
| `fixtures/events/` | Sample session for the browser replay and the tests, built locally and not committed |
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

Open <http://localhost:1420>. Outside Tauri, the app replays `fixtures/events/sample-session.json` as if it were happening live. The header has replay speed buttons (1, 4, and 20 times) and **Restart**. Inside the Tauri window, the same UI listens for events from Rust instead and shows **Live**.

If the header shows **Replay finished 0:00 / 0:00** and the views are empty, the sample session is missing. The developer console also logs `[corvid] No sample session`. Run step 2, then restart `npm run dev`.

## Event contract

`desktop/src/data/event.ts` defines the events Rust sends to the UI. Rust emits batches (`CorvidEvent[]`) on the Tauri event `corvid://events`, with camelCase field names. On the Rust side, use `#[serde(tag = "type", rename_all = "camelCase")]`. The UI drops events that don't match the contract and logs a `[corvid]` warning in the developer console, so check the console first if events don't appear.

## Sample session

`scripts/make-sample-session.mjs` builds the sample session from a real combat log. It keeps only events involving the player who recorded the log and the mobs they fought, and drops everyone else. The recording player's name and GUID are kept.

The script writes `fixtures/events/sample-session.json` unless you pass a different output path as a second argument. It also shortens idle stretches so the replay doesn't sit still between fights. Each developer builds their own, and `fixtures/events/` is gitignored, so the file is never committed.

Don't commit raw combat logs either. They contain other players' names. `WTF/` and `WoWCombatLog*.txt` are gitignored.

## Tests

From `desktop/`:

```powershell
npm test
```

This reruns the tests each time you save a file. To run them once, use `npx vitest run`. Tests live in `desktop/tests/` as `*.test.ts` files and cover the event contract, text and number formatting, fight totals, and map trails. They need neither the game nor Rust.

Most tests use small made-up events. Four also run on your sample session, if you've built one, and check things that hold for any log, such as meter totals matching the raw events. Without a sample session, as in CI, those four are skipped.

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

Run the desktop app separately in PowerShell. The combat log is the first data source the desktop app will read.

## Build checks

Run from `desktop/`:

```powershell
npx vitest run
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug --no-bundle
```

The first command runs the tests once. The second checks TypeScript, including the tests, and builds the Vite frontend into `dist/`. The Cargo command checks the Rust backend and fails if Cargo would need to change the lockfile. The last command builds the Windows debug executable without packaging an installer. To build a release installer, use `npm run tauri build`.

## CI

`.github/workflows/ci.yml` runs on pull requests, on pushes to `main`, and when started manually from the **Actions** tab. It has two jobs:

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
