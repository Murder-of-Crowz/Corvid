import { useMemo, useState } from "react";
import { useEventFeed } from "../data/store";
import {
  boundsOf,
  buildMapState,
  encounterMarks,
  trailSegments,
  type Bounds,
  type EncounterMark,
  type Track,
} from "../data/tracks";
import { formatClock, formatCoord } from "../lib/format";
import { buildFights } from "../data/fights";

/** Drawn tat the game's world map size, so 0-1 coordinates keep the zone' shape. */
const MAP_WIDTH = 1002;
const MAP_HEIGHT = 668;

const TRAIL_BREAK_MS = 30_000;
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

type Zoom = "close" | "zone";

const ZOOMS: { id: Zoom; label: string }[] = [
  { id: "close", label: "Close-up" },
  { id: "zone", label: "Whole zone" },
];

interface ViewBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

const ZONE_VIEW: ViewBox = { x: 0, y: 0, w: MAP_WIDTH, h: MAP_HEIGHT };

/** Your trail and pin on the zone you are in, plus party member's pins. */
export function MapView() {
  const { events } = useEventFeed();
  const [zoom, setZoom] = useState<Zoom>("close");
  const state = useMemo(() => buildMapState(events), [events]);
  const marks = useMemo(
    () => (state ? encounterMarks(buildFights(events), state.self.points) : []),
    [events, state],
  );

  if (!state) {
    return (
      <p className="view-empty">
        No position yet. Positions come from the combat log, so yours appears once you're in combat.
      </p>
    );
  }

  const map = MAPS[state.mapId] ?? { name: `Map ${state.mapId}` };
  const image = zoneImage(map.zoneId);
  const here = state.self.points[state.self.points.length -1];
  const everyone = [state.self, ...state.others];
  const bounds = boundsOf(everyone.flatMap((track) => track.points));
  const view = zoom === "close" && bounds ? closeUpView(bounds) : ZONE_VIEW;
  const unit = view.w / MAP_WIDTH;
  const step = zoom === "close" ? 0.01 : 0.1;

  return (
    <section className="map">
      <div className="map-toolbar">
        <span className="map-title">{map.name}</span>
        <span className="muted">
          {formatCoord(here.x)}, {formatCoord(here.y)} &middot; {formatClock(here.ts)}
        </span>
        <div className="map-zooms" role="group" aria-label="Zoom">
          {ZOOMS.map((z) => (
            <button
              key={z.id}
              type="button"
              className="chip"
              aria-pressed={zoom === z.id}
              onClick={() => setZoom(z.id)}
            >
              {z.label}
            </button>
          ))}
        </div>
      </div>

      <svg
        className="map-canvas"
        viewBox={`${view.x} ${view.y} ${view.w} ${view.h}`}
        role="img"
        aria-label={`Map of ${map.name} with your trail`}
      >
        {image && <image href={image} x={0} y={0} width={MAP_WIDTH} height={MAP_HEIGHT} />}

        <rect className="map-edge" x={0} y={0} width={MAP_WIDTH} height={MAP_HEIGHT} />
        {!image && (
          <>
            {gridLines(view.x / MAP_WIDTH, (view.x + view.w) / MAP_WIDTH, step).map((x) => (
              <line key={`x${x}`} className="map-grid" x1={x * MAP_WIDTH} x2={x * MAP_WIDTH} y1={0} y2={MAP_HEIGHT} />
            ))}
            {gridLines(view.y / MAP_HEIGHT, (view.y + view.h) / MAP_HEIGHT, step).map((y) => (
              <line key={`y${y}`} className="map-grid" x1={0} x2={MAP_WIDTH} y1={y * MAP_HEIGHT} y2={y * MAP_HEIGHT} />
            ))}
          </>
        )}

        {state.others.map((track) => (
          <TrackMarks key={track.unit.guid} track={track} unit={unit} className="map-party" />
        ))}
        <TrackMarks track={state.self} unit={unit} className="map-self"  marks={marks} />
      </svg>
    </section>
  );
}

interface TrackMarksProps {
  track: Track;
  unit: number;
  className: string;
  marks?: EncounterMark[];
}

/** A unit's trail, a pin where it is now, and numbered encounters. */
function TrackMarks({ track, unit, className, marks = [] }: TrackMarksProps) {
  const segments = trailSegments(track.points, TRAIL_BREAK_MS);
  const last = track.points[track.points.length - 1];
  return (
    <g className={className}>
      {segments.slice(1).map((segment, i) => {
        const from = segments[i][segments[i].length -1];
        const to = segment[0];
        return (
          <line
            key={`gap${to.ts}`}
            className="map-gap"
            x1={from.x * MAP_WIDTH}
            y1={from.y * MAP_HEIGHT}
            x2={to.x * MAP_WIDTH}
            y2={to.y * MAP_HEIGHT}
          />
        );
      })}
      {segments.map((segment) => (
        <polyline
          key={segment[0].ts}
          className="map-trail"
          points={segment.map((p) => `${p.x * MAP_WIDTH},${p.y * MAP_HEIGHT}`).join(" ")}
        />
      ))}
      <circle className="map-pin" cx={last.x * MAP_WIDTH} cy={last.y * MAP_HEIGHT} r={7 * unit}>
        <title>{`${track.unit.name} ${formatCoord(last.x)}, ${formatCoord(last.y)}`}</title>
      </circle>
      {marks.map((mark, i) => (
        <g
          key={mark.number}
          className={i === marks.length -1 ? "map-mark map-mark-latest" : "map-mark"}
          transform={`translate(${mark.x * MAP_WIDTH} ${mark.y * MAP_HEIGHT}) scale(${unit})`}
        >
          <title>{`${mark.number}. ${mark.name}`}</title>
          <circle r={9} />
          <text dy="0.35em">{mark.number}</text>
        </g>
      ))}
    </g>
  );
}

/** A box around the trail, at least CLOSE_MIN_SPAN across, in the map's shape. */
function closeUpView(bounds: Bounds): ViewBox {
  let w = (Math.max(bounds.maxX - bounds.minX, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) * MAP_WIDTH;
  let h = (Math.max(bounds.maxY - bounds.minY, CLOSE_MIN_SPAN) + 2 * CLOSE_PADDING) * MAP_HEIGHT;
  if (w / h < MAP_WIDTH / MAP_HEIGHT) w = (h * MAP_WIDTH) / MAP_HEIGHT;
  else h = (w * MAP_HEIGHT) / MAP_WIDTH;
  const centerX = ((bounds.minX + bounds.maxX) / 2) * MAP_WIDTH;
  const centerY = ((bounds.minY + bounds.maxY) / 2) * MAP_HEIGHT;
  return { x: centerX - w /2, y: centerY - h / 2, w, h };
}

/** Grid position (0-1) every `step` between start and end, kept inside the map. */
function gridLines(start: number, end: number, step: number): number[] {
  const lines: number[] = [];
  const first = Math.ceil(Math.max(0, start) / step);
  const last = Math.floor(Math.min(1, end) / step);
  for (let i = first; i <= last; i++) lines.push(i * step);
  return lines;
}