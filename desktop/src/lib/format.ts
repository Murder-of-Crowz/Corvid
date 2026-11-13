import type { CombatEvent, CorvidEvent } from "../data/event";

const clock = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
  hourCycle: "h23",
});
const whole = new Intl.NumberFormat();

/** Local time of day */
export function formatClock(ts: number): string {
  return clock.format(ts);
}

/** 12345 > 12,345 */
export function formatNumber(n: number): string {
  return whole.format(n);
}

/** Seconds as "m:ss", i.e 83 > 1:23 */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** A 0-1 map coordiantes the way the game shows it: i.e. 0.3301 > "33.0" */
export function formatCoord(value: number): string {
  return (value * 100).toFixed(1);
}

/** A per-sec rate: 1 decimal below 100, whole numbers above. 7.24 > "7.2", 1234.5 > "1,235". */
export function formatRate(perSecond: number): string {
  return perSecond < 100 ? perSecond.toFixed(1) : formatNumber(Math.round(perSecond));
}

/** A 0-1 share as a whole percentage, i.e. 0.256 > "26%" */
export function formatPercent(share: number): string {
  return `${Math.round(share * 100)}%`
}

/** The last part of a folder path, i.e "C:\Games\Logs" > "Logs" */
export function folderName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** 1 line of plain text describing the event. */
export function describe(event: CorvidEvent): string {
  switch (event.type) {
    case "position":
      return `${event.unit.name} at ${formatCoord(event.x)}, ${formatCoord(event.y)} (map ${event.mapId})`;
    case "fightStart":
      return `Fight started: ${event.name}${event.zone ? ` in ${event.zone}` : ""}`;
    case "fightEnd":
      return `Fight ended${event.success === true ? ": won" : event.success === false ? ": wiped" : ""}`;
    case "combat":
      return describeCombat(event);
    case "chat":
      return `[${event.channel}] ${event.sender ?? "System"}: ${event.message}`;
    case "rawLine":
      return event.line;
  }
}

function describeCombat(event: CombatEvent): string {
  switch (event.kind) {
    case "damage":
      return (
        `${event.source.name} hit ${event.target.name} for ${formatNumber(event.amount)}` +
        (event.spell ? ` with ${event.spell.name}` : " in melee") +
        notes(
          event.critical ? "critical" : undefined,
          event.overkill ? `${formatNumber(event.overkill)} overkill` : undefined,
        )
      );
    case "heal":
      return (
        `${event.source.name} healed ${event.target.name} for ${formatNumber(event.amount)}` +
        ` with ${event.spell.name}` +
        notes(
          event.critical ? "critical" : undefined,
          event.overheal ? `${formatNumber(event.overheal)} overheal` : undefined,
        )
      );
    case "death":
      return `${event.target.name} died`;
  }
}

/** " (a, b)" from the parts that are present or "" when none are. */
function notes(...parts: (string | undefined)[]): string {
  const present = parts.filter((part): part is string => part !== undefined);
  return present.length > 0 ? ` (${present.join(", ")})` : "";
}