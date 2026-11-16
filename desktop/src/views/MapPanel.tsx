import { useState } from "react";
import { LiveMapView } from "./LiveMapView";
import { MapView } from "./MapView";

type MapTab = "live" | "fights";

const TABS: { id: MapTab; label: string }[] = [
  { id: "live", label: "Live" },
  { id: "fights", label: "Fights" },
];

/** The map panel: where you are now, or your trail and fights. */
export function MapPanel() {
  const [tab, setTab] = useState<MapTab>("live");
  return (
    <section className="map">
      <div className="map-tabs" role="tablist" aria-label="Map">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            className="map-tab"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "live" ? <LiveMapView /> : <MapView />}
    </section>
  )
}