# WoW Forever Companion MVP Build Plan

After Phase 0, build the smallest end-to-end version first: read a combat event from WoW and show it in the desktop app. Then add features one at a time, with a completion check for each milestone.

## 1. Repository and event contract · A few days

- [ ] Set up the monorepo with a Tauri app, React UI, Lua addon folder, and test data folder.
- [ ] Agree on example `position`, `combat`, and `chat` events, including timestamps, identifiers, and optional fields. Use these examples as fixtures so Rust and React work can proceed in parallel.

**Done when** both developers can build their parts against the same event examples.

## 2. First end-to-end slice · 1 week

- [ ] **Rust:** Read new lines from `WoWCombatLog.txt` starting at the last read position and send them to the UI.
- [ ] **React:** Show a scrolling list of raw events in the Tauri window.

**Done when** you hit a mob in game and see the corresponding line in the app while WoW is still running. Record how long it takes to appear so you can set expectations for the meter.

## 3. Combat parser and damage meter · 2–3 weeks

- [ ] **Rust:** Parse comma-separated fields with a CSV parser that handles quoted names. Handle damage, healing, deaths, and available encounter markers. Define and test how fights start and end when the log has no explicit boundary.
- [ ] **Rust:** Use the Phase 0 samples to test damage and healing totals, paired swing records, and fight grouping.
- [ ] **React:** Show per-player damage, healing, and DPS. Refresh the display a few times per second as new events arrive.

**Done when** a recorded fight produces the expected totals and a live fight updates the meter as WoW writes events. This is the largest Rust milestone.

## 4. SQLite and 24-hour retention · 1 week

- [ ] **Rust:** Store fights and parsed events in SQLite with batched writes. Automatically remove data older than 24 hours.
- [ ] **React:** Show a list of recent fights.

**Done when** fights are still listed after restarting the app and records past the cutoff are removed.

## 5. Map from combat log positions · 2 weeks

- [ ] Start with one zone and one map image.
- [ ] **Rust:** Convert combat log world coordinates to normalized map coordinates (`0–1`). Calibrate the conversion at known locations.
- [ ] **React:** Draw a position pin and a trail from the available samples.
- [ ] Add more zones after the first zone passes calibration.

**Done when** the pin and trail line up with known locations in the first zone. Use combat log positions for this milestone and add continuous movement in step 7.

## 6. Chat · 1 week

- [ ] **Rust:** Read and parse new chat log lines. Store them locally for replay under the same 24-hour retention rule. This is a good first Rust task for the developer who is newer to it.
- [ ] **React:** Add a read-only chat panel with channel filters. Hide whispers by default.

**Done when** supported channels appear in the panel and whispers stay hidden until enabled.

## 7. Optional pixel strip · 2 weeks

- [ ] **Lua:** Encode the map ID and readable player coordinates in the visible strip with a checksum.
- [ ] **Rust:** Capture only the strip's screen region and reject frames that fail validation.
- [ ] **React:** Use decoded samples to update the same pin and trail outside combat.

**Done when** the pin follows the player during ordinary movement without combat events. Keep this optional and validate it after the log-based map is working.

## 8. Fight replay · 1–2 weeks

- [ ] Let the user select a fight from the last 24 hours.
- [ ] Replay damage, available position samples, and chat on a shared timeline.

**Done when** selecting a saved fight lets the user review all three streams in sync, including fights that only have combat log positions.

## 9. Packaging and private test · 1–2 weeks

- [ ] Package the Windows app and bundle addon installation and updates.
- [ ] Set up the desktop auto-updater and sign the installer.
- [ ] Run a private test with a few players and record issues before public release.

**Done when** a tester can install the app and addon on another PC, generate a combat event, and see it in the app.

## Team split

| Developer | Primary work |
| --- | --- |
| Person A | Rust file reader, combat parser, SQLite, and map coordinate conversion. |
| Person B | React UI for the meter, map, chat, and replay. Own the Lua addon and take the chat parser as a first Rust task. |
| Both | Agree on the event contract, validate the first end-to-end slice, and run the private test. |

## Schedule

Plan for roughly **12–16 weeks of part-time work for two people**, assuming some Rust and UI work overlaps. Use the repository setup and first end-to-end slice to recalibrate the milestone estimates before committing to a release date.
