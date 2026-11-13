import { useEffect } from "react";
import { ChooseLogsDir } from "./components/ChooseLogsDir";
import { Header } from "./components/Header";
import { startFeed } from "./data/feed";
import { useLogsDir } from "./data/logsDir";
import { EventsView } from "./views/EventsView";
import { MapView } from "./views/MapView";
import { MeterView } from "./views/MeterView";
import "./App.css";

export default function App() {
  const logsDir = useLogsDir();

  useEffect(() => startFeed(), []);

  return (
    <div className="app">
      <Header logsDir={logsDir} />
      {logsDir.path === null ? (
        <main className="app-main">
          <ChooseLogsDir onChoose={logsDir.choose} />
        </main>
      ) : (
        <main className="dashboard">
          <MapView />
          <div className="side">
            <EventsView />
            <MeterView />
          </div>
        </main>
      )}
    </div>
  )
}