import { useMemo, useState } from "react";
import { MapToolbar, ZoneMap, type Zoom } from "../components/ZoneMap";
import { useEventFeed } from "../data/store";
import {
  boundsOf,
  buildMapState,
  encounterMarks,
  trailSegments,
  type EncounterMark,
  type Track,
} from "../data/tracks";
import { closeUpView, MAP_HEIGHT, MAP_WIDTH, ZONE_VIEW, zoneFor } from "../data/zones";
import { formatCoord } from "../lib/format";
import { buildFights } from "../data/fights";

const TRAIL_BREAK_MS = 30_000;

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

  const zone = zoneFor(state.mapId);
  const here = state.self.points[state.self.points.length - 1];
  const everyone = [state.self, ...state.others];
  const bounds = boundsOf(everyone.flatMap((track) => track.points));
  const view = zoom === "close" && bounds ? closeUpView(bounds) : ZONE_VIEW;
  const unit = view.w / MAP_WIDTH;

  return (
    <>
      <MapToolbar title={zone.name} here={here} zoom={zoom} onZoom={setZoom} />
      <ZoneMap image={zone.image} view={view} label={`Map of ${zone.name} with your trail`}>
        {state.others.map((track) => (
          <TrackMarks key={track.unit.guid} track={track} unit={unit} className="map-party" />
        ))}
        <TrackMarks track={state.self} unit={unit} className="map-self" marks={marks} />
      </ZoneMap>
    </>
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