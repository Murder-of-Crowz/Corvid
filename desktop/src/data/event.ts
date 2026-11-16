/**
 * Corvid event contract: the messages the Rust backend sends to the UI.
 * 
 * Rust emits batches on the Tauri event `corvid://events`. Each payload is a
 * `CorvidEvent[]`, so the UI refreshes per batch instead of per event. Field
 * names are camelCase on the wire. On the Rust side, use
 * `#[serde(tag = "type", rename_all = "camelCase")]`.
 */

export const EVENTS_CHANNEL = "corvid://events";

/**
 * Unix epoc ms, UTC. Log timestamps carry their own UTC offset
 * (i.e. "11/5/2026 23:17:29.943-8"), so Rust converst them w/o guessing.
 */
export type Timestamp = number;

export type UnitKind = "player" | "pet" | "npc";

/** From combat log unit flags. "self" is the player running the app (flags 0x511). */
export type Affiliation = "self" | "party" | "raid" | "outsider";

/** The four parts of a player name */
export interface PlayerName {
  firstName: string;
  surname: string;
  mode: string;
  region: string;
}

export interface Unit {
  guid: string;
  name: string;
  kind: UnitKind;
  affiliation: Affiliation;
  playerName?: PlayerName;
  ownerGuid?: string;
}

export interface Spell {
  id: number;
  name: string;
}

export interface PositionEvent {
  type: "position";
  ts: Timestamp;
  unit: Unit;
  /** 0 when there's no position to show, like a party member who left your map. */
  mapId: number;
  x: number;
  y: number;
  source: "strip" | "log";
}

export interface FightStartEvent {
  type: "fightStart";
  ts: Timestamp;
  fightId: string;
  kind: "trash" | "boss";
  name: string;
  zone?: string;
  mapId?: number;
}

export interface FightEndEvent {
  type: "fightEnd";
  ts: Timestamp;
  fightId: string;
  success?: boolean;
}

interface CombatBase {
  type: "combat";
  ts: Timestamp;
  fightId?: string;
}

export interface DamageEvent extends CombatBase {
  kind: "damage",
  source: Unit;
  target: Unit;
  spell?: Spell;
  amount: number;
  overkill?: number;
  critical?: boolean;
}

export interface HealEvent extends CombatBase {
  kind: "heal";
  source: Unit;
  target: Unit;
  spell: Spell;
  amount: number;
  overheal?: number;
  critical?: boolean;
}

export interface DeathEvent extends CombatBase {
  kind: "death";
  target: Unit;
}

export type CombatEvent = DamageEvent | HealEvent | DeathEvent;

export type ChatChannel =
  | "say" | "yell" | "emote"
  | "party" | "raid" | "guild" | "officer"
  | "whisper" | "trade" | "general" | "system" | "other";

export interface ChatEvent {
  type: "chat";
  ts: Timestamp;
  channel: ChatChannel;
  sender?: string;
  recipient?: string;
  message: string;
}

/**
 * An unparsed log line. LEts the first e2e slice ship before
 * the parser exists, and later helps debug lines the parser skips.
 */
export interface RawLineEvent {
  type: "rawLine";
  ts: Timestamp;
  log: "combat" | "chat";
  line: string;
}

export type CorvidEvent =
  | PositionEvent
  | FightStartEvent
  | FightEndEvent
  | CombatEvent
  | ChatEvent
  | RawLineEvent;

const EVENT_TYPES = new Set<string>([
  "position", "fightStart", "fightEnd", "combat", "chat", "rawLine"
]);
const COMBAT_KINDS = new Set<string>(["damage", "heal", "death"]);

/**
 * Shallow check at the boundary with Rust and the fixtures. it catches
 * contract drift without validating every field.
 */
export function isCorvidEvent(value: unknown): value is CorvidEvent {
  if (typeof value !== "object" || value === null) return false;
  const event = value as Record<string, unknown>;
  if (typeof event.ts !== "number") return false;
  if (typeof event.type !== "string" || !EVENT_TYPES.has(event.type)) return false;
  if (event.type === "combat") {
    return typeof event.kind === "string" && COMBAT_KINDS.has(event.kind);
  }
  return true;
}

/** Keeps valid events and logs how many were dropped. */
export function toEvents(values: unknown, origin: string): CorvidEvent[] {
  if (!Array.isArray(values)) {
    console.warn(`[corvid] ${origin}: expected an array of events`);
    return [];
  }
  const events = values.filter(isCorvidEvent);
  const dropped = values.length - events.length;
  if (dropped > 0) {
    console.warn(`[corvid] ${origin}: dropped ${dropped} event(s) that don't match the contract`)
  }
  return events;
}