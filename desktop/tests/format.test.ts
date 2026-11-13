import { describe, expect, it } from "vitest";
import type { Unit } from "../src/data/event";
import {
  describe as describeEvent,
  folderName,
  formatCoord,
  formatDuration,
  formatPercent,
  formatRate,
} from "../src/lib/format";

const me: Unit = { guid: "Player-1", name: "Anslem Nightglyph", kind: "player", affiliation: "self" };
const mob: Unit = { guid: "Creature-1", name: "Mindless Zombie", kind: "npc", affiliation: "outsider" };

describe("describe", () => {
  it("names the spell, crit and overkill on damage", () => {
    const text = describeEvent({
      type: "combat", ts: 0, kind: "damage", source: me, target: mob,
      spell: { id: 133, name: "Fireball" }, amount: 20, overkill: 8, critical: true,
    });
    expect(text).toBe("Anslem Nightglyph hit Mindless Zombie for 20 with Fireball (critical, 8 overkill)");
  });

  it("calls spell-less damage melee", () => {
    const text = describeEvent({ type: "combat", ts: 0, kind: "damage", source: mob, target: me, amount: 2 });
    expect(text).toBe("Mindless Zombie hit Anslem Nightglyph for 2 in melee");
  });

  it("describes heals, deaths and fights", () => {
    const heal = describeEvent({
      type: "combat", ts: 0, kind: "heal", source: me, target: me,
      spell: { id: 1, name: "Touch of the Grave" }, amount: 6, overheal: 6,
    });
    expect(heal).toBe("Anslem Nightglyph healed Anslem Nightglyph for 6 with Touch of the Grave (6 overheal)");
    expect(describeEvent({ type: "combat", ts: 0, kind: "death", target: mob })).toBe("Mindless Zombie died");
    expect(describeEvent({ type: "fightEnd", ts: 0, fightId: "f", success: false })).toBe("Fight ended: wiped");
  });
});

describe("number formats", () => {
  it("formats durations as m:ss", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(83.9)).toBe("1:23");
    expect(formatDuration(-5)).toBe("0:00");
  });

  it("formats coordinates the way the game does", () => {
    expect(formatCoord(0.3301)).toBe("33.0");
  });

  it("formats rates and shares", () => {
    expect(formatRate(7.24)).toBe("7.2");
    expect(formatRate(1234.5)).toMatch(/^1\D?235$/); // the separator depends on locale
    expect(formatPercent(0.256)).toBe("26%");
  });
});

describe("folderName", () => {
  it("keeps the last part of Windows and Unix paths", () => {
    expect(folderName(String.raw`D:\Games\WoW Forever\Logs`)).toBe("Logs");
    expect(folderName("D:\\Games\\Logs\\")).toBe("Logs");
    expect(folderName("/home/me/wow/Logs")).toBe("Logs");
    expect(folderName("C:\\")).toBe("C:");
  });
});
