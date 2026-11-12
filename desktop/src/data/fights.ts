import type { CombatEvent, CorvidEvent, Unit } from "./event";

export interface Fight {
  id: string;
  name: string;
  kind: "trash" | "boss";
  zone?: string;
  start: number;
  end?: number;
  success?: boolean;
  events: CombatEvent[];
}

/** Groups the feed's combat events into fights, oldest first. */
export function buildFights(events: readonly CorvidEvent[]): Fight[] {
  const fights: Fight[] = [];
  const byId = new Map<string, Fight>();

  // A fight can be seen before its fightStart, if the start was trimmed from the feed.
  function fightFor(id: string, ts: number): Fight {
    let fight = byId.get(id);
    if (!fight) {
      fight = { id, name: "Unknown", kind: "trash", start: ts, events: [] };
      byId.set(id, fight);
      fights.push(fight);
    }
    return fight;
  }

  for (const event of events) {
    switch (event.type) {
      case "fightStart" : {
        const fight = fightFor(event.fightId, event.ts);
        fight.name = event.name;
        fight.kind = event.kind;
        fight.zone = event.zone;
        fight.start = event.ts;
        break;
      }
      case "fightEnd": {
        const fight = fightFor(event.fightId, event.ts);
        fight.end = event.ts;
        fight.success = event.success;
        break;
      }
      case "combat":
        if (event.fightId) fightFor(event.fightId, event.ts).events.push(event);
        break;
    }
  }
  return fights;
}

/**
 * Seconds the fight lasted, or has lasted so far. Measured from event
 * timestamps, never Date.now(), so it's right during a fast replay too.
 */
export function fightDuration(fight: Fight): number {
  const last = fight.end ?? fight.events[fight.events.length - 1]?.ts ?? fight.start;
  return Math.max(0, last - fight.start) / 1000;
}

export type MeterMode = "damage" | "healing";

export interface SpellTotal {
  key: string;
  name: string;
  total: number;
  hits: number;
  crits: number;
}

export interface MeterRow {
  unit: Unit;
  total: number;
  perSecond: number;
  spells: SpellTotal[];
}

/** Per-sec rates use at least this, so a 1-ht fight does not divide by zero. */
const MIN_RATE_SECONDS = 1;

/**
 * Dmg or healing done by your side (you, party, raid and pets) in one fight,
 * highest firs. Overkill & overheal are left out, so totals match what actually
 * came off or went onto health bars.
 */
export function meterRows(fight: Fight, mode: MeterMode): MeterRow[] {
  const wanted = mode === "damage" ? "damage" : "heal";
  const rows = new Map<string, { unit: Unit; total: number; spells: Map<String, SpellTotal> }>();

  for (const event of fight.events) {
    if (event.kind === "death" || event.kind !== wanted) continue;
    const source = event.source;
    if (source.affiliation === "outsider") continue;

    const isPet = source.kind === "pet" && source.ownerGuid !== undefined;
    const rowKey = isPet ? source.ownerGuid! : source.guid;
    let row = rows.get(rowKey);
    if (!row) {
      row = { unit: source, total: 0, spells: new Map() };
      rows.set(rowKey, row);
    } else if (!isPet && row.unit.kind === "pet") {
      row.unit = source;
    }

    const wasted = event.kind === "damage" ? event.overkill : event.overheal;
    const amount = event.amount - (wasted ?? 0);
    row.total += amount;

    const spellKey = (isPet ? `${source.guid}/` : "") + (event.spell?.id ?? "melee");
    const spellName = (event.spell?.name ?? "Melee") + (isPet ? ` (${source.name})` : "");
    let spell = row.spells.get(spellKey);
    if (!spell) {
      spell = { key: spellKey, name: spellName, total: 0, hits: 0, crits: 0 };
      row.spells.set(spellKey, spell);
    }
    spell.total += amount;
    spell.hits += 1;
    if (event.critical) spell.crits += 1;
  }

  const seconds = Math.max(fightDuration(fight), MIN_RATE_SECONDS);
  return [...rows.values()]
    .map((row) => ({
      unit: row.unit,
      total: row.total,
      perSecond: row.total / seconds,
      spells: [...row.spells.values()].sort((a, b) => b.total - a.total),
    }))
    .sort((a, b) => b.total - a.total);
}