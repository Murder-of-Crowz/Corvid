import { useEffect } from "react";
import { ChooseWowDir } from "./components/ChooseWowDir";
import { Header } from "./components/Header";
import { startFeed } from "./data/feed";
import { useWowDir } from "./data/wowDir";
import { EventsView } from "./views/EventsView";
import { MapPanel } from "./views/MapPanel";
import { MeterView } from "./views/MeterView";
import "./App.css";


export default function App() {
  const wowDir = useWowDir();

  useEffect(() => startFeed(), []);

  return (
    <div className="app">
      <Header wowDir={wowDir} />
      {wowDir.path === null ? (
        <main className="app-main">
          <ChooseWowDir onChoose={wowDir.choose} />
        </main>
      ) : (
        <main className="dashboard">
          <MapPanel />
          <div className="side">
            <MeterView />
            <EventsView />
          </div>
        </main>
      )}
    </div>
  )
}