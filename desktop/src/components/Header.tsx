import { REPLAY_SPEEDS, replay, useReplayStatus } from "../data/feed";
import { LogsDir } from "../data/logsDir";
import { folderName, formatDuration } from "../lib/format";

export type View = "meter" | "map" | "events";

const VIEWS: { id: View, label: string }[] = [
  { id: "meter", label: "Meter" },
  { id: "map", label: "Map" },
  { id: "events", label: "Events" },
];

interface HeaderProps {
  view: View;
  onViewChange: (view: View) => void;
  logsDir: LogsDir;
}

/** Title and view tabs, plus replay controls in the browser or a live badge in the app. */
export function Header({ view, onViewChange, logsDir }: HeaderProps) {
  const status = useReplayStatus();

  return (
    <header className="header">
      <h1 className="header-title">Corvid</h1>

      <nav className="tabs" aria-label="Views">
        {VIEWS.map((v) => (
          <button
            key={v.id}
            type="button"
            className="tab"
            aria-current={view === v.id ? "page" : undefined}
            onClick={() => onViewChange(v.id)}
          >
            {v.label}
          </button>
        ))}
      </nav>

      {replay && status ? (
        <div className="replay" role="group" aria-label="Replay">
          <span className="muted">{status.done ? "Replay finished" : "Replaying sample"}</span>
          <span className="replay-time">
            {formatDuration(status.elapsed)} / {formatDuration(status.duration)}
          </span>
          {REPLAY_SPEEDS.map((speed) => (
            <button
              key={speed}
              type="button"
              className="chip"
              aria-pressed={status.speed === speed}
              onClick={() => replay?.setSpeed(speed)}
            >
              {speed}*
            </button>
          ))}
          <button type="button" className="button" onClick={replay.restart}>
            Restart
          </button>
        </div>
      ) : logsDir.path ? (
        <div className="live">
          <span className="live-badge">Live</span>
          <button
            type="button"
            className="link"
            title={`Reading combat logs from ${logsDir.path}. Click to change.`}
            onClick={logsDir.choose}
          >
            {folderName(logsDir.path)}
          </button>
        </div>
      ) : null}
    </header>
  )
}