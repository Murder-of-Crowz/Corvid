/** Builds the UI's replay fixture from a real WoW: Forever combat log.
 * 
 * Keeps only the events involving the player who recorded the log (unit flags 0x511)
 * and the mobs they fought, so other players' names never reach the repo.
 * 
 * Usage, from the repo root:
 *  node scripts/make-sample-session.mjs WTF/CombatLog/WoWCombatLog-110526_231729.txt (example)
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const [input, output = "fixtures/events/sample-session.json"] = process.argv.slice(2);
if (!input) {
  console.error("usage: node scripts/make-sample-session.mjs <combat log> [output.json]");
  process.exit(1);
}

/** A fight ends after this long with no damage to or from the player. */
const FIGHT_GAP_MS = 6000;
/** Quiet stretches longer than this are shortened. Must exceed FIGHT_GAP_MS so fights stay intact. */
const MAX_IDLE_MS = 8000;
/** At most one position per unit per interval. */
const POSITION_INTERVAL_MS = 1000;

// Unit flags (COMBATLOG_OBJECT_*).
const AFFILIATION_MINE = 0x1;
const AFFILIATION_PARTY = 0x2;
const AFFILIATION_RAID = 0x4;
const TYPE_PLAYER = 0x400;
const TYPE_PET = 0x1000;

/** Advanced params are 19 fields: infoGUID, ownerGUID, hp, maxHp, ap, sp, armor,
 * absorb, 2 unknown, powerType, power, maxPower, cost, posX, posY, mapId, facing, level. */
const ADVANCED_LENGTH = 19;
const DAMAGE = new Set(["SWING_DAMAGE", "RANGE_DAMAGE", "SPELL_DAMAGE", "SPELL_PERIODIC_DAMAGE"]);
const HEAL = new Set(["SPELL_HEAL", "SPELL_PERIODIC_HEAL"]);
const POSITION_ONLY = new Set(["SWING_DAMAGE_LANDED", "SPELL_CAST_SUCCESS", "SPELL_ENERGIZE"]);

/** Swings have no spell prefix, so their advanced params start 3 fields earlier. */
const advancedStart = (e) => (e.startsWith("SWING_") ? 9 : 12);

/** "11/5/2026 23:17:29.943-8" -> epoch ms. The trailing number is the UTC offset in hours. */
function parseTimestamp(text) {
  const m = /^(\d+)\/(\d+)\/(\d+) (\d+):(\d+):(\d+)\.(\d+)([-+]\d+(?:\.\d+)?)$/.exec(text);
  if (!m) return null;
  const [, month, day, year, h, min, s, frac, offset] = m;
  const ms = Number(frac.padEnd(3, "0").slice(0, 3));
  return Date.UTC(+year, +month - 1, +day, +h, +min, +s, ms) - Number(offset) * 3_600_000;
}

/** Splits on commas outside double quotes and drops the quotes. */
function splitFields(text) {
  const fields = [];
  let current = "";
  let quoted = false;
  for (const ch of text) {
    if (ch === '"') quoted = !quoted;
    else if (ch === "," && !quoted) {
      fields.push(current);
      current = "";
    } else current += ch;
  }
  fields.push(current);
  return fields;
}

function parseLine(line) {
  const split = line.indexOf("  ");
  if (split < 0) return null;
  const ts = parseTimestamp(line.slice(0, split));
  return ts === null ? null : { ts, fields: splitFields(line.slice(split + 2)) };
}

function toUnit(guid, rawName, flagsHex) {
  const flags = parseInt(flagsHex, 16);
  const unit = {
    guid,
    name: rawName,
    kind: flags & TYPE_PLAYER ? "player" : flags & TYPE_PET ? "pet" : "npc",
    affiliation:
      flags & AFFILIATION_MINE ? "self"
      : flags & AFFILIATION_PARTY ? "party"
      : flags & AFFILIATION_RAID ? "raid"
      : "outsider",
  };
  const parts = rawName.split("-");
  if (guid.startsWith("Player-") && parts.length === 4) {
    const [firstName, surname, mode, region] = parts;
    unit.name = `${firstName} ${surname}`;
    unit.playerName = { firstName, surname, mode, region };
  }
  return unit;
}

const round4 = (n) => Math.round(n * 10_000) / 10_000;

const lines = readFileSync(input, "utf8").split(/\r?\n/);
// Sorting matters: after a logging rs, the game flushes older buffered lines late.
const records = lines.map(parseLine).filter(Boolean).sort((a, b) => a.ts - b.ts);

const events = [];
const mapBounds = new Map(); // mapId > [x0, x1, y0, y1] from MAP_CHANGE
const lastPositionAt = new Map(); // Unit GUID > ts
let zone;
let mapId;
let fight = null; // { id, lastTs, enemies: Set<GUID> }
let fightCount = 0

function startFight(ts, enemy) {
  fightCount += 1;
  fight = { id: `f-${String(fightCount).padStart(4, "0")}`, lastTs: ts, enemies: new Set() };
  events.push({ type: "fightStart", ts, fightId: fight.id, kind: "trash", name: enemy.name, zone, mapId });
}

function endFight() {
  events.push({ type: "fightEnd", ts: fight.lastTs + 1, fightId: fight.id });
  fight = null;
}

function addPosition(ts, unit, posX, posY, onMap) {
  const bounds = mapBounds.get(onMap);
  if (!bounds || ts - (lastPositionAt.get(unit.guid) ?? -Infinity) < POSITION_INTERVAL_MS) return;
  lastPositionAt.set(unit.guid, ts);
  mapId = onMap;
  const [x0, x1, y0, y1] = bounds;
  events.push({
    type: "position",
    ts,
    unit,
    mapId: onMap,
    x: round4((y0 - posY) / (y0 - y1)),
    y: round4((x0 - posX) / (x0 - x1)),
    source: "log",
  });
}

for (const { ts, fields: f } of records) {
  const event = f[0];
  if (event === "ZONE_CHANGE") {
    zone = f[2];
    continue;
  }
  if (event === "MAP_CHANGE") {
    mapBounds.set(Number(f[1]), f.slice(3, 7).map(Number));
    mapId ??= Number(f[1]);
    continue;
  }
  if (fight && ts - fight.lastTs > FIGHT_GAP_MS) endFight();
  if (f.length < 9) continue;

  const source = toUnit(f[1], f[2], f[3]);
  const target = toUnit(f[5], f[6], f[7]);
  const selfIsSource = source.affiliation === "self";
  const selfIsTarget = target.affiliation === "self";

  // The advanced params desc the unit named by infoGUID, which may be either side.
  if (DAMAGE.has(event) || HEAL.has(event) || POSITION_ONLY.has(event)) {
    const a = advancedStart(event);
    const info = f[a];
    const infoUnit = info === source.guid ? source : info === target.guid ? target : null;
    if (infoUnit?.affiliation === "self" && f.length >= a + ADVANCED_LENGTH) {
      addPosition(ts, infoUnit, Number(f[a + 14]), Number(f[a + 15]), Number(f[a + 16]));
    }
  }

  if (DAMAGE.has(event) && (selfIsSource || selfIsTarget)) {
    const enemy = selfIsSource ? target : source;
    if (!fight) startFight(ts, enemy);
    fight.lastTs = ts;
    fight.enemies.add(enemy.guid);
    const s = advancedStart(event) + ADVANCED_LENGTH;
    const damage = { type: "combat", ts, fightId: fight.id, kind: "damage", source, target, amount: Number(f[s]) };
    if (!event.startsWith("SWING_")) damage.spell = { id: Number(f[9]), name: f[10] };
    if (Number(f[s + 2]) > 0) damage.overkill = Number(f[s + 2]);
    if (f[s + 7] === "1") damage.critical = true;
    events.push(damage);
  } else if (HEAL.has(event) && (selfIsSource || selfIsTarget)) {
    const s = 12 + ADVANCED_LENGTH;
    const heal = {
      type: "combat", ts, ...(fight && { fightId: fight.id }), kind: "heal",
      source, target, spell: { id: Number(f[9]), name: f[10] }, amount: Number(f[s]),
    };
    if (Number(f[s + 2]) > 0) heal.overheal = Number(f[s + 2]);
    if (f[s + 4] === "1") heal.critical = true;
    events.push(heal);
  } else if (event === "UNIT_DIED" && (selfIsTarget || fight?.enemies.has(target.guid))) {
    if (fight) fight.lastTs = ts;
    events.push({ type: "combat", ts, ...(fight && { fightId: fight.id }), kind: "death", target });
  }
}
if (fight) endFight();

// Shorten long idle stretches so the replay does NOT sit still for minutes.
events.sort((a, b) => a.ts - b.ts);
let removed = 0;
let previous = events[0]?.ts ?? 0;
for (const event of events) {
  const gap = event.ts - previous;
  previous = event.ts;
  if (gap > MAX_IDLE_MS) removed += gap - MAX_IDLE_MS;
  event.ts -= removed;
}

mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, "[\n" + events.map((e) => "  " + JSON.stringify(e)).join(",\n") + "\n]\n");

const span = events.length ? (events.at(-1).ts - events[0].ts) / 1000 : 0;
console.log(`${events.length} events, ${fightCount} fights, ${span.toFixed(0)} s replay -> ${output}`);