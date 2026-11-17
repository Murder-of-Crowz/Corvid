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
  1411: { name: "Durotar", zoneId: 14 },
  1412: { name: "Mulgore", zoneId: 215 },
  1413: { name: "The Barrens", zoneId: 17 },
  1416: { name: "Alterac Mountains", zoneId: 36 },
  1417: { name: "Arathi Highlands", zoneId: 45 },
  1418: { name: "Badlands", zoneId: 3 },
  1419: { name: "Blasted Lands", zoneId: 4 },
  1420: { name: "Tirisfal Glades", zoneId: 85 },
  1421: { name: "Silverpine Forest", zoneId: 130 },
  1422: { name: "Western Plaguelands", zoneId: 28 },
  1423: { name: "Eastern Plaguelands", zoneId: 139 },
  1424: { name: "Hillsbrad Foothills", zoneId: 267 },
  1425: { name: "The Hinterlands", zoneId: 47 },
  1426: { name: "Dun Morogh", zoneId: 1 },
  1427: { name: "Searing Gorge", zoneId: 51 },
  1428: { name: "Burning Steppes", zoneId: 46 },
  1429: { name: "Elwynn Forest", zoneId: 12 },
  1430: { name: "Deadwind Pass", zoneId: 41 },
  1431: { name: "Duskwood", zoneId: 10 },
  1432: { name: "Loch Modan", zoneId: 38 },
  1433: { name: "Redridge Mountains", zoneId: 44 },
  1434: { name: "Stranglethorn Vale", zoneId: 33 },
  1435: { name: "Swamp of Sorrows", zoneId: 8 },
  1436: { name: "Westfall", zoneId: 40 },
  1437: { name: "Wetlands", zoneId: 11 },
  1438: { name: "Teldrassil", zoneId: 141 },
  1439: { name: "Darkshore", zoneId: 148 },
  1440: { name: "Ashenvale", zoneId: 331 },
  1441: { name: "Thousand Needles", zoneId: 400 },
  1442: { name: "Stonetalon Mountains", zoneId: 406 },
  1443: { name: "Desolace", zoneId: 405 },
  1444: { name: "Feralas", zoneId: 357 },
  1445: { name: "Dustwallow Marsh", zoneId: 15 },
  1446: { name: "Tanaris", zoneId: 440 },
  1447: { name: "Azshara", zoneId: 16 },
  1448: { name: "Felwood", zoneId: 361 },
  1449: { name: "Un'Goro Crater", zoneId: 490 },
  1450: { name: "Moonglade", zoneId: 493 },
  1451: { name: "Silithus", zoneId: 1377 },
  1452: { name: "Winterspring", zoneId: 618 },
  1453: { name: "Stormwind City", zoneId: 1519 },
  1454: { name: "Orgrimmar", zoneId: 1637 },
  1455: { name: "Ironforge", zoneId: 1537 },
  1456: { name: "Thunder Bluff", zoneId: 1638 },
  1457: { name: "Darnassus", zoneId: 1657 },
  1458: { name: "Undercity", zoneId: 1497 },
  1459: { name: "Alterac Valley", zoneId: 2597 },
  1461: { name: "Arathi Basin", zoneId: 3358 },
  2482: { name: "Mount Hyjal", zoneId: 616 },
  2521: { name: "Zephras Isle", zoneId: 16593 },
  2524: { name: "Darkspear Islands", zoneId: 16606 },
  2548: { name: "Riverglades", zoneId: 16591 },
  2652: { name: "Shen'dralas", zoneId: 16651 },
  2665: { name: "Zephras Isle", zoneId: 16593 },
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
  return Object.entries(ZONE_IMAGES).find(([path]) =>
    path.includes(prefix),
  )?.[1];
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
  let w =
    (Math.max(bounds.maxX - bounds.minX, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) *
    MAP_WIDTH;
  let h =
    (Math.max(bounds.maxY - bounds.minY, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) *
    MAP_HEIGHT;
  if (w / h < MAP_WIDTH / MAP_HEIGHT) w = (h * MAP_WIDTH) / MAP_HEIGHT;
  else h = (w * MAP_HEIGHT) / MAP_WIDTH;
  const centerX = ((bounds.minX + bounds.maxX) / 2) * MAP_WIDTH;
  const centerY = ((bounds.minY + bounds.maxY) / 2) * MAP_HEIGHT;
  return { x: centerX - w / 2, y: centerY - h / 2, w, h };
}
