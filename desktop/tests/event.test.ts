import { describe, expect, it, vi } from "vitest";
import { isCorvidEvent, toEvents } from "../src/data/event";

describe("isCorvidEvent", () => {
  it("accepts every event type", () => {
    expect(isCorvidEvent({ type: "position", ts: 1 })).toBe(true);
    expect(isCorvidEvent({ type: "fightStart", ts: 1 })).toBe(true);
    expect(isCorvidEvent({ type: "fightEnd", ts: 1 })).toBe(true);
    expect(isCorvidEvent({ type: "combat", ts: 1, kind: "damage" })).toBe(true);
    expect(isCorvidEvent({ type: "chat", ts: 1 })).toBe(true);
    expect(isCorvidEvent({ type: "rawLine", ts: 1 })).toBe(true);
  });

  it("rejects values that break the contract", () => {
    expect(isCorvidEvent(null)).toBe(false);
    expect(isCorvidEvent("combat")).toBe(false);
    expect(isCorvidEvent({ type: "position" })).toBe(false); // no ts
    expect(isCorvidEvent({ type: "teleport", ts: 1 })).toBe(false);
    expect(isCorvidEvent({ type: "combat", ts: 1, kind: "absorb" })).toBe(false);
    expect(isCorvidEvent({ type: "combat", ts: 1 })).toBe(false); // no kind
  });
});

describe("toEvents", () => {
  it("keeps valid events, drops the rest, and warns", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const events = toEvents([{ type: "fightEnd", ts: 1, fightId: "f" }, { type: "nope", ts: 2 }], "test");
    expect(events).toHaveLength(1);
    expect(warn).toHaveBeenCalledOnce();
    warn.mockRestore();
  });

  it("returns nothing when the payload isn't an array", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(toEvents({ type: "fightEnd", ts: 1 }, "test")).toEqual([]);
    warn.mockRestore();
  });
});
