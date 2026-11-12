import { ComponentType, useEffect, useState } from "react";
import { Header, type View } from "./components/Header";
import { startFeed } from "./data/feed";
import { EventsView } from "./views/EventsView";
import { MapView } from "./views/MapView";
import { MeterView } from "./views/MeterView";
import "./App.css";

const VIEW_COMPONENTS: Record<View, ComponentType> = {
  meter: MeterView,
  map: MapView,
  events: EventsView,
};

export default function App() {
  const [view, setView] = useState<View>("meter");
  const CurrentView = VIEW_COMPONENTS[view]

  // Start the feed once; React calls the returned function to stop it.
  useEffect(() => startFeed(), []);

  return (
    <div className="app">
      <Header view={view} onViewChange={setView} />
      <main className="app-main">
        <CurrentView />
      </main>
    </div>
  )
}