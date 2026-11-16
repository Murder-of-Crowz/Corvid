import type { CorvidEvent, PositionEvent, Unit } from "./event";
import type { Fight } from "./fights";

export interface TrackPoint {
  ts: number;
  x: number;
  y: number;
}

export interface Track {
  unit: Unit;
  points: TrackPoint[];
}

export interface MapState {
  mapId: number;
  self: Track;
  others: Track[];
}

export function latestPosition(events: readonly CorvidEvent[]): PositionEvent | null {
  for (let i = events.length - 1; i >= 0; i--) {
    const event = events[i];
    if (event.type === "position" && event.unit.affiliation === "self" && event.unit.kind === "player") return event;
  }
  return null;
}

/**
 * Where you and your group have been on the map you're on now. Null
 * until your first positon arrives. Pets and strangers are left out.
 */
export function buildMapState(events: readonly CorvidEvent[]): MapState | null {
  const mapId = latestPosition(events)?.mapId;
  if (mapId === undefined) return null;

  const tracks = new Map<string, Track>();
  for (const event of events) {
    if (event.type !== "position" || event.mapId !== mapId) continue;
    if (event.unit.kind !== "player" || event.unit.affiliation === "outsider") continue;
    let track = tracks.get(event.unit.guid);
    if (!track) {
      track = { unit: event.unit, points: [] };
      tracks.set(event.unit.guid, track);
    }
    track.points.push({ ts: event.ts, x: event.x, y: event.y });
  }

  const all = [...tracks.values()];
  const self = all.find((track) => track.unit.affiliation === "self");
  return self ? { mapId, self, others: all.filter((track) => track !== self) } : null;
}

/**
 * Splits a trail where sampels are far apart in time. Combat log positions only arrive
 * during combat, so a straight line across a long gap would cut across w/e path was really taken.
 */
export function trailSegments(points: readonly TrackPoint[], maxGapMs: number): TrackPoint[][] {
  const segments: TrackPoint[][] = [];
  let current: TrackPoint[] = [];
  for (const point of points) {
    const prev = current[current.length - 1];
    if (prev && point.ts - prev.ts > maxGapMs) {
      segments.push(current);
      current = [];
    }
    current.push(point);
  }
  if (current.length > 0) segments.push(current);
  return segments;
}

export interface EncounterMark {
  number: number;
  name: string;
  x: number;
  y: number;
}

/**
 * Where you were when each fight started, numbered from 1 for the oldest. A 
 * fight with none of your positions from just before it on is left out, but keeps
 * its number. Positions come at most once a second, so the one taken at the fight's 
 * first hit can be up to a second early.
 */
export function encounterMarks(fights: readonly Fight[], points: readonly TrackPoint[]): EncounterMark[] {
  const marks: EncounterMark[] = [];
  fights.forEach((fight, i) => {
    const end = fight.end ?? Infinity;
    const point = points.find((p) => p.ts >= fight.start - 1000 && p.ts <= end);
    if (point) marks.push({ number: i + 1, name: fight.name, x: point.x, y: point.y });
  });
  return marks;
}

export interface Bounds {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

/** The smallest box around every point, for zooming in on the trail. */
export function boundsOf(points: readonly TrackPoint[]): Bounds | null {
  if (points.length === 0) return null;
  const bounds = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
  for (const { x, y } of points) {
    bounds.minX = Math.min(bounds.minX, x);
    bounds.minY = Math.min(bounds.minY, y);
    bounds.maxX = Math.max(bounds.maxX, x);
    bounds.maxY = Math.max(bounds.maxY, y);
  }
  return bounds;
}