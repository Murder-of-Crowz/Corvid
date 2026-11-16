import { afterEach, describe, expect, it } from "vitest";
import type { CorvidEvent, Unit } from "../src/data/event";
import { ReplaySource } from "../src/data/feed";
import { eventStore } from "../src/data/store";

const me: Unit = { guid: "Player-1", name: "Anslem Nightglyph", kind: "player", affiliation: "self" };
const mob: Unit = { guid: "Creature-1", name: "Mindless Zombie", kind: "npc", affiliation: "outsider" };

/** A fight with one hit, starting `ts` ms into the log. */
function fight(id: string, ts: number): CorvidEvent[] {
  return [
    { type: "fightStart", ts, fightId: id, kind: "trash", name: mob.name },
    { type: "combat", kind: "damage", ts, fightId: id, source: me, target: mob, amount: 10 },
    { type: "fightEnd", ts: ts + 1, fightId: id },
  ];
}

// Three fights a minute apart. The second is flushed late, like after a log restart.
const LOG = [...fight("f-1", 0), ...fight("f-3", 120_000), ...fight("f-2", 60_000)];

/** The fight IDs the store has seen start. */
function startedFights(): string[] {
  return eventStore
    .getSnapshot()
    .events.flatMap((event) => (event.type === "fightStart" ? [event.fightId] : []));
}

describe("ReplaySource", () => {
  const replay = new ReplaySource(async () => structuredClone(LOG));
  afterEach(() => replay.close());

  it("plays from the start and counts the encounters", async () => {
    await replay.open("test.txt");
    replay.pause();
    expect(replay.getStatus()).toMatchObject({ log: "test.txt", loading: false, encounter: 0, encounters: 3 });
    expect(startedFights()).toEqual([]);
  });

  it("jumps between encounters in time order", async () => {
    await replay.open("test.txt");
    replay.pause();

    replay.nextEncounter();
    replay.nextEncounter();
    expect(replay.getStatus()?.encounter).toBe(2);
    expect(startedFights()).toEqual(["f-1", "f-2"]);

    replay.nextEncounter();
    replay.nextEncounter(); // already at the last
    expect(replay.getStatus()?.encounter).toBe(3);

    replay.prevEncounter();
    expect(replay.getStatus()?.encounter).toBe(2);
    expect(startedFights()).toEqual(["f-1", "f-2"]);

    replay.prevEncounter();
    replay.prevEncounter(); // from the first, back to the start of the log
    expect(replay.getStatus()?.encounter).toBe(0);
    expect(startedFights()).toEqual([]);
    expect(replay.getStatus()?.playing).toBe(false);
  });

  it("has no status once closed", async () => {
    await replay.open("test.txt");
    replay.close();
    expect(replay.getStatus()).toBeNull();
  });
});
