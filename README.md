# Corvid

Corvid is a Windows desktop companion and Lua addon for World of Warcraft: Forever. We’re building a combat log reader, a damage and healing meter, and a map that shows positions from combat events. The goal is to keep recent fights locally so players can review them after combat.

The desktop app currently opens a window that says **Corvid**. The addon attempts to enable advanced combat logging on world entry, and `/corvid` checks which player and party positions the game returns. The next milestone is to show new combat log lines in the desktop window while WoW is running.

| Folder | Contents |
| --- | --- |
| `addon/` | `Corvid.toc` and `Corvid.lua` |
| `desktop/src/` | React + TypeScript UI, built with Vite |
| `desktop/src-tauri/` | Rust backend and Tauri configuration |

## Tech stack and pinned versions

| Component | Version |
| --- | --- |
| Node.js / npm | 26.11.1 / 11.20.0 |
| Rust / Cargo | 1.99.1, Windows x64 MSVC toolchain |
| React / React DOM | 19.3.1 / 19.3.1 |
| TypeScript / React type packages | 6.0.3 / 19.3.0 |
| Vite / React plugin | 8.3.6 / 6.2.1 |
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
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug --no-bundle
```

The first command checks TypeScript and builds the Vite frontend into `dist/`. The Cargo command checks the Rust backend and fails if Cargo would need to change the lockfile. The last command builds the Windows debug executable without packaging an installer. To build a release installer, use `npm run tauri build`.

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
