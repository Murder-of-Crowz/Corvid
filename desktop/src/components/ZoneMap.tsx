import type { ReactNode } from "react";
import type { TrackPoint } from "../data/tracks";
import { MAP_HEIGHT, MAP_WIDTH, type ViewBox } from "../data/zones";
import { formatClock, formatCoord } from "../lib/format";

export type Zoom = "close" | "zone";

const ZOOMS: { id: Zoom; label: string}[] = [
  { id: "close", label: "Close-up" },
  { id: "zone", label: "Whole zone" },
];

interface MapToolbarProps {
  title: string;
  here: TrackPoint;
  zoom: Zoom;
  onZoom: (zoom: Zoom) => void;
}

/** The zone's name, where you are and when, and the zoom buttons. */
export function MapToolbar({ title, here, zoom, onZoom }: MapToolbarProps) {
  return (
    <div className="map-toolbar">
      <span className="map-title">{title}</span>
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
            onClick={() => onZoom(z.id)}
          >
            {z.label}
          </button>
        ))}
      </div>
    </div>
  )
}

interface ZoneMapProps {
  image?: string;
  view: ViewBox;
  label: string;
  children: ReactNode;
}

/** The zone's map image, or a grid where there isn't one, under whatever a view draws. */
export function ZoneMap({ image, view, label, children }: ZoneMapProps) {
  const step = view.w < MAP_WIDTH ? 0.01 : 0.1;
  return (
    <svg
      className="map-canvas"
      viewBox={`${view.x} ${view.y} ${view.w} ${view.h}`}
      role="img"
      aria-label={label}
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
      {children}
    </svg>
  )
}

/** Grid position (0-1) every `step between start and end, kept inside the map. */
function gridLines(start: number, end: number, step: number): number[] {
  const lines: number[] = [];
  const first = Math.ceil(Math.max(0, start) / step);
  const last = Math.floor(Math.min(1, end) / step);
  for (let i = first; i <= last; i++) lines.push(i * step);
  return lines;
}