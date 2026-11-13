import { ComponentType, useEffect, useState } from "react";
import { Header, type View } from "./components/Header";
import { startFeed } from "./data/feed";
import { EventsView } from "./views/EventsView";
import { MapView } from "./views/MapView";
import { MeterView } from "./views/MeterView";
import "./App.css";
import { useLogsDir } from "./data/logsDir";
import { ChooseLogsDir } from "./components/ChooseLogsDir";

const VIEW_COMPONENTS: Record<View, ComponentType> = {
  meter: MeterView,
  map: MapView,
  events: EventsView,
};

export default function App() {
  const [view, setView] = useState<View>("meter");
  const CurrentView = VIEW_COMPONENTS[view]
  const logsDir = useLogsDir();

  // Start the feed once; React calls the returned function to stop it.
  useEffect(() => startFeed(), []);

  return (
    <div className="app">
      <Header view={view} onViewChange={setView} logsDir={logsDir} />
      <main className="app-main">
        {logsDir.path === null ? <ChooseLogsDir onChoose={logsDir.choose} /> : <CurrentView />}
      </main>
    </div>
  )
}