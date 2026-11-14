import { describe, expect, it } from "vitest";
import type { CorvidEvent, Unit } from "../src/data/event";
import type { Fight } from "../src/data/fights";
import { boundsOf, buildMapState, encounterMarks, trailSegments } from "../src/data/tracks";
import { sampleEvents } from "./sample";

describe("buildMapState", () => {
  // The sample comes from each developer's own log, so this checks shape, not exact numbers.
  it.skipIf(!sampleEvents)("builds your trail from the sample session", () => {
    const events = sampleEvents ?? [];
    const state = buildMapState(events)!;
    expect(state).not.toBeNull();
    expect(state.self.unit.affiliation).toBe("self");
    const yours = events.filter(
      (event) => event.type === "position" && event.mapId === state.mapId && event.unit.guid === state.self.unit.guid,
    );
    expect(state.self.points).toHaveLength(yours.length);
    for (const track of state.others) {
      expect(["party", "raid"]).toContain(track.unit.affiliation);
    }
  });

  it("is null before your first position, without throwing", () => {
    expect(buildMapState([])).toBeNull();
  });

  it("keeps only players in your group on your current map", () => {
    const unit = (guid: string, kind: Unit["kind"], affiliation: Unit["affiliation"]): Unit =>
      ({ guid, name: guid, kind, affiliation });
    const at = (ts: number, who: Unit, mapId: number): CorvidEvent =>
      ({ type: "position", ts, unit: who, mapId, x: 0.5, y: 0.5, source: "log" });
    const me = unit("me", "player", "self");
    const friend = unit("friend", "player", "party");

    const state = buildMapState([
      at(1, friend, 1420),
      at(2, me, 1426), // an earlier map
      at(3, unit("stranger", "player", "outsider"), 1420),
      at(4, unit("pet", "pet", "self"), 1420),
      at(5, friend, 1426), // friend on another map
      at(6, me, 1420),
    ])!;
    expect(state.mapId).toBe(1420);
    expect(state.self.points.map((p) => p.ts)).toEqual([6]);
    expect(state.others.map((t) => [t.unit.guid, t.points.map((p) => p.ts)])).toEqual([["friend", [1]]]);
  });
});

describe("trailSegments", () => {
  it("splits where samples are further apart than the gap", () => {
    const points = [0, 1000, 2000, 40_000, 41_000].map((ts) => ({ ts, x: 0, y: 0 }));
    expect(trailSegments(points, 30_000).map((s) => s.length)).toEqual([3, 2]);
    expect(trailSegments([], 30_000)).toEqual([]);
  });
});

describe("boundsOf", () => {
  it("finds the box around the points", () => {
    const points = [
      { ts: 0, x: 0.33, y: 0.69 },
      { ts: 1, x: 0.37, y: 0.65 },
    ];
    expect(boundsOf(points)).toEqual({ minX: 0.33, minY: 0.65, maxX: 0.37, maxY: 0.69 });
    expect(boundsOf([])).toBeNull();
  });
});

describe("encounterMarks", () => {
  it("numbers each fight from the oldest, where you were when it started", () => {
    const fight = (name: string, start: number, end?: number): Fight =>
      ({ id: name, name, kind: "trash", start, end, events: [] });
    const points = [
      { ts: 9_500, x: 0.1, y: 0.1 }, // half a second before the first hit
      { ts: 12_000, x: 0.2, y: 0.2 },
      { ts: 70_000, x: 0.5, y: 0.5 },
    ];
    const marks = encounterMarks(
      [
        fight("Zombie", 10_000, 15_000),
        fight("Bat", 40_000, 45_000), // no position during it, so no mark
        fight("Spider", 70_000), // still going
      ],
      points,
    );
    expect(marks).toEqual([
      { number: 1, name: "Zombie", x: 0.1, y: 0.1 },
      { number: 3, name: "Spider", x: 0.5, y: 0.5 },
    ]);
  });
});
