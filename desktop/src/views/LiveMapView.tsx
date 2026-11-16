import { type ReactNode, useState } from "react";
import { MapToolbar, ZoneMap, type Zoom } from "../components/ZoneMap";
import type { PositionEvent } from "../data/event";
import { useLiveHeading, useLiveParty, useLivePosition } from "../data/feed";
import { boundsOf } from "../data/tracks";
import { closeUpView, MAP_HEIGHT, MAP_WIDTH, ZONE_VIEW, zoneFor } from "../data/zones";
import { formatCoord } from "../lib/format";

/** An arrowhead pointing up, with a notch in its back. (0, 0) is where you are. */
const ARROW = "M0 -11 L8 9 L0 4 L-8 9 Z";

/** Where you are now, only your pin. It stays live even while a replay is showing. */
export function LiveMapView() {
  const here = useLivePosition();
  const heading = useLiveHeading();
  const party = useLiveParty();
  const [zoom, setZoom] = useState<Zoom>("close");

  if (!here) {
    return (
      <p className="view-empty">
        No live position yet. Yours appears once the game is showing with the Corvid addon, or when you next fight or cast.
      </p>
    );
  }

  const zone = zoneFor(here.mapId);
  // Only party members on your map. Close up, the view fits all of you.
  const nearby = party.filter((mbr) => mbr.mapId === here.mapId);
  const bounds = boundsOf([here, ...nearby]);
  const view = zoom === "close" && bounds ? closeUpView(bounds) : ZONE_VIEW;
  const unit = view.w / MAP_WIDTH;

  return (
    <>
      <MapToolbar title={zone.name} here={here} zoom={zoom} onZoom={setZoom} />
      <ZoneMap image={zone.image} view={view} label={`Map of ${zone.name} with where you and your party are`}>
        <g className="map-party">
          {nearby.map((mbr) => (
            <Marker key={mbr.unit.guid} position={mbr} unit={unit}>
              <circle className="map-pin" r={7} />
              <text className="map-name" y={-12}>
                {mbr.unit.name}
              </text>
            </Marker>
          ))}
        </g>
        <g className="map-self">
          <Marker position={here} unit={unit}>
            <path className="map-pin map-arrow" d={ARROW} transform={`rotate(${heading ?? 0})`} />
          </Marker>
        </g>
      </ZoneMap>
    </>
  );
}

interface MarkerProps {
  position: PositionEvent;
  unit: number;
  children: ReactNode;
}

/**
 * Draws its children at `position`, scaled so they're the same size on screen
 * at any zoom. Hovering shows the name and coordinates.
 */
function Marker({ position, unit, children }: MarkerProps) {
  return (
    <g transform={`translate(${position.x *MAP_WIDTH} ${position.y * MAP_HEIGHT}) scale(${unit})`}>
      <title>{`${position.unit.name} ${formatCoord(position.x)}, ${formatCoord(position.y)}`}</title>
      {children}
    </g>
  )
}