import { afterEach, describe, expect, it, vi } from "vitest";
import { zoneFor } from "../src/data/zones";

describe("zoneFor", () => {
  afterEach(() => {
    vi.doUnmock("@tauri-apps/api/core");
    vi.resetModules();
  });

  it("names a map by the uiMapID the combat log uses", () => {
    expect(zoneFor(1420).name).toBe("Tirisfal Glades");
  });

  it("names a map it doesn't know by its ID", () => {
    expect(zoneFor(99999).name).toBe("Map 99999");
  });

  // Map pictures are drawn from the player's own game files, so none ship with
  // Corvid. Outside the app there are no game files, so there's no picture.
  it("has no picture outside the app", () => {
    expect(zoneFor(1420).image).toBeUndefined();
  });

  // The app answers http://map.localhost/<uiMapID>.png. It draws any map the
  // game has, named or not.
  it("asks the app's map protocol for the picture by uiMapID", async () => {
    vi.doMock("@tauri-apps/api/core", () => ({
      isTauri: () => true,
      convertFileSrc: (path: string, protocol: string) => `${protocol}://${path}`,
    }));
    const zones = await import("../src/data/zones");
    expect(zones.zoneFor(1420).image).toBe("map://1420.png");
    expect(zones.zoneFor(99999).image).toBe("map://99999.png");
  });
});
