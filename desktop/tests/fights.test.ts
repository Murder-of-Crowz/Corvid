import { describe, expect, it } from "vitest";
import type { CorvidEvent, Unit } from "../src/data/event";
import { buildFights, fightDuration, meterRows } from "../src/data/fights";
import { sampleEvents } from "./sample";

// Each developer builds the sample from their own combat log, so these checks
// hold for any log instead of expecting exact numbers.
describe.skipIf(!sampleEvents)("the sample session", () => {
  const events = sampleEvents ?? [];
  const fights = buildFights(events);

  it("has fights, and every one ended", () => {
    expect(fights.length).toBeGreaterThan(0);
    expect(fights.every((fight) => fight.end !== undefined)).toBe(true);
  });

  it("totals the same effective damage as the raw events", () => {
    let raw = 0;
    for (const event of events) {
      if (event.type === "combat" && event.kind === "damage" && event.fightId && event.source.affiliation !== "outsider") {
        raw += event.amount - (event.overkill ?? 0);
      }
    }
    let summed = 0;
    for (const fight of fights) {
      for (const row of meterRows(fight, "damage")) summed += row.total;
    }
    expect(raw).toBeGreaterThan(0);
    expect(summed).toBe(raw);
  });

  it("keeps every per-second rate finite, even for very short fights", () => {
    for (const fight of fights) {
      for (const row of meterRows(fight, "damage")) expect(Number.isFinite(row.perSecond)).toBe(true);
    }
  });
});

describe("meterRows", () => {
  const me: Unit = { guid: "Player-1", name: "Me", kind: "player", affiliation: "self" };
  const pet: Unit = { guid: "Pet-1", name: "Wolf", kind: "pet", affiliation: "self", ownerGuid: "Player-1" };
  const mob: Unit = { guid: "Creature-1", name: "Mob", kind: "npc", affiliation: "outsider" };
  const fight = buildFights([
    { type: "fightStart", ts: 0, fightId: "f", kind: "trash", name: "Mob" },
    { type: "combat", ts: 100, fightId: "f", kind: "damage", source: pet, target: mob, amount: 5 },
    { type: "combat", ts: 200, fightId: "f", kind: "damage", source: me, target: mob, spell: { id: 1, name: "Bolt" }, amount: 10, critical: true },
    { type: "combat", ts: 300, fightId: "f", kind: "damage", source: mob, target: me, amount: 7 },
    { type: "combat", ts: 400, fightId: "f", kind: "heal", source: me, target: me, spell: { id: 2, name: "Mend" }, amount: 9, overheal: 4 },
    { type: "combat", ts: 450, fightId: "f", kind: "damage", source: me, target: mob, spell: { id: 1, name: "Bolt" }, amount: 6, overkill: 2 },
    { type: "combat", ts: 460, fightId: "f", kind: "death", target: mob },
  ] satisfies CorvidEvent[])[0];

  it("merges a pet into its owner, even when the pet hits first", () => {
    const rows = meterRows(fight, "damage");
    expect(rows).toHaveLength(1); // the mob's hit on you isn't counted
    expect(rows[0].unit.name).toBe("Me");
    expect(rows[0].spells.map((spell) => spell.name)).toEqual(["Bolt", "Melee (Wolf)"]);
    expect(rows[0].spells[0].hits).toBe(2);
    expect(rows[0].spells[0].crits).toBe(1);
  });

  it("leaves overkill out of damage and overheal out of healing", () => {
    expect(meterRows(fight, "damage")[0].total).toBe(19); // 5 + 10 + (6 - 2)
    expect(meterRows(fight, "healing")[0].total).toBe(5); // 9 - 4
  });

  it("counts a fight under a second as one second, so the rate stays sane", () => {
    expect(fightDuration(fight)).toBeLessThan(1);
    const [row] = meterRows(fight, "damage");
    expect(row.perSecond).toBe(row.total);
  });
});
