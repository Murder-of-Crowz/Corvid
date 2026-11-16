import type { Bounds } from "./tracks";

export const MAP_WIDTH = 1002;
export const MAP_HEIGHT = 668;

const CLOSE_MIN_SPAN = 0.12;
const CLOSE_PADDING = 0.01;

interface MapInfo {
  name: string;
  zoneId?: number;
}

const MAPS: Record<number, MapInfo> = {
  1420: { name: "Tirisfal Glades", zoneId: 85 },
  1421: { name: "Silverpine Forest", zoneId: 130 },
  1458: { name: "Undercity", zoneId: 1497 },
};

/**
 * Zone maps exported from the game with wow.export (for testing), which names
 * them Zone_<zoneId>_<name>.png. They're Blizzard's art, so they stay in the
 * .gitignored WTF/Zones folder. A zone without one is drawn on a plain grid.
 */
const ZONE_IMAGES = import.meta.glob<string>("../../../WTF/Zones/Zone_*.png", {
  eager: true,
  query: "?url",
  import: "default",
});

function zoneImage(zoneId: number | undefined): string | undefined {
  if (zoneId === undefined) return undefined;
  const prefix = `/Zone_${zoneId}_`;
  return Object.entries(ZONE_IMAGES).find(([path]) => path.includes(prefix))?.[1];
}

export interface Zone {
  name: string;
  image?: string;
}

/** A map's name and exported image, by the uiMapID the combat log uses. */
export function zoneFor(mapId: number): Zone {
  const map = MAPS[mapId] ?? { name: `Map ${mapId}` };
  return { name: map.name, image: zoneImage(map.zoneId) };
}

export interface ViewBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const ZONE_VIEW: ViewBox = { x: 0, y: 0, w: MAP_WIDTH, h: MAP_HEIGHT };

/** A box around the bounds, at least CLOSE_MIN_SPAN across, in the map's shape */
export function closeUpView(bounds: Bounds): ViewBox {
  let w = (Math.max(bounds.maxX - bounds.minX, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) * MAP_WIDTH;
  let h = (Math.max(bounds.maxY - bounds.minY, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) * MAP_HEIGHT;
  if (w / h < MAP_WIDTH / MAP_HEIGHT) w = (h * MAP_WIDTH) / MAP_HEIGHT;
  else h = (w * MAP_HEIGHT) / MAP_WIDTH;
  const centerX = ((bounds.minX + bounds.maxX) / 2) * MAP_WIDTH;
  const centerY = ((bounds.minY + bounds.maxY) / 2) * MAP_HEIGHT;
  return { x: centerX - w / 2, y: centerY - h / 2, w, h };
}