import { useState } from "react";
import { MapToolbar, ZoneMap, type Zoom } from "../components/ZoneMap";
import { useLivePosition } from "../data/feed";
import { closeUpView, MAP_HEIGHT, MAP_WIDTH, ZONE_VIEW, zoneFor } from "../data/zones";
import { formatCoord } from "../lib/format";

/** Where you are now, only your pin. It stays live even while a replay is showing. */
export function LiveMapView() {
  const here = useLivePosition();
  const [zoom, setZoom] = useState<Zoom>("close");

  if (!here) {
    return (
      <p className="view-empty">
        No live position yet. Yours appears once the game is showing with the Corvid addon, or when you next fight or cast.
      </p>
    );
  }

  const zone = zoneFor(here.mapId);
  const view =
    zoom === "close" ? closeUpView({ minX: here.x, maxX: here.x, minY: here.y, maxY: here.y }) : ZONE_VIEW;
  const unit = view.w / MAP_WIDTH;

  return (
    <>
      <MapToolbar title={zone.name} here={here} zoom={zoom} onZoom={setZoom} />
      <ZoneMap image={zone.image} view={view} label={`Map of ${zone.name} with where you are`}>
        <g className="map-self">
          <circle className="map-pin" cx={here.x * MAP_WIDTH} cy={here.y * MAP_HEIGHT} r={7 * unit}>
            <title>{`${here.unit.name} ${formatCoord(here.x)}, ${formatCoord(here.y)}`}</title>
          </circle>
        </g>
      </ZoneMap>
    </>
  );
}