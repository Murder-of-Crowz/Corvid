import type { Bounds } from "./tracks";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";

export const MAP_WIDTH = 1002;
export const MAP_HEIGHT = 668;

const CLOSE_MIN_SPAN = 0.12;
const CLOSE_PADDING = 0.01;

interface MapInfo {
  name: string;
}

const MAPS: Record<number, MapInfo> = {
  1411: { name: "Durotar" },
  1412: { name: "Mulgore" },
  1413: { name: "The Barrens" },
  1416: { name: "Alterac Mountains" },
  1417: { name: "Arathi Highlands" },
  1418: { name: "Badlands" },
  1419: { name: "Blasted Lands"},
  1420: { name: "Tirisfal Glades" },
  1421: { name: "Silverpine Forest" },
  1422: { name: "Western Plaguelands" },
  1423: { name: "Eastern Plaguelands" },
  1424: { name: "Hillsbrad Foothills" },
  1425: { name: "The Hinterlands" },
  1426: { name: "Dun Morogh" },
  1427: { name: "Searing Gorge" },
  1428: { name: "Burning Steppes" },
  1429: { name: "Elwynn Forest" },
  1430: { name: "Deadwind Pass" },
  1431: { name: "Duskwood" },
  1432: { name: "Loch Modan" },
  1433: { name: "Redridge Mountains" },
  1434: { name: "Stranglethorn Vale" },
  1435: { name: "Swamp of Sorrows" },
  1436: { name: "Westfall" },
  1437: { name: "Wetlands" },
  1438: { name: "Teldrassil" },
  1439: { name: "Darkshore" },
  1440: { name: "Ashenvale" },
  1441: { name: "Thousand Needles" },
  1442: { name: "Stonetalon Mountains" },
  1443: { name: "Desolace" },
  1444: { name: "Feralas" },
  1445: { name: "Dustwallow Marsh" },
  1446: { name: "Tanaris" },
  1447: { name: "Azshara" },
  1448: { name: "Felwood" },
  1449: { name: "Un'Goro Crater" },
  1450: { name: "Moonglade" },
  1451: { name: "Silithus" },
  1452: { name: "Winterspring" },
  1453: { name: "Stormwind City" },
  1454: { name: "Orgrimmar" },
  1455: { name: "Ironforge" },
  1456: { name: "Thunder Bluff" },
  1457: { name: "Darnassus" },
  1458: { name: "Undercity" },
  1459: { name: "Alterac Valley" },
  1461: { name: "Arathi Basin" },
  2482: { name: "Mount Hyjal" },
  2521: { name: "Zephras Isle" },
  2524: { name: "Darkspear Islands" },
  2548: { name: "Riverglades" },
  2652: { name: "Shen'dralas" },
  2665: { name: "Zephras Isle" },
};

/**
 * The map's picture. The app draws it from the player's own game files, so
 * none of Blizzard's art ship with Corvid. Outside the app there's none.
 */
function mapImage(mapId: number): string | undefined {
  return isTauri() ? convertFileSrc(`${mapId}.png`, "map") : undefined;
}

export interface Zone {
  name: string;
  image?: string;
}

/** A map's name and exported image, by the uiMapID the combat log uses. */
export function zoneFor(mapId: number): Zone {
  const map = MAPS[mapId] ?? { name: `Map ${mapId}` };
  return { name: map.name, image: mapImage(mapId) };
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
