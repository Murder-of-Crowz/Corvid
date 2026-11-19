import { REPLAY_SPEEDS, replay, useReplayStatus, watch, type ReplayStatus } from "../data/feed";
import { useCombatLogs, type WowDir } from "../data/wowDir";
import { folderName, formatClock, logLabel } from "../lib/format";

interface HeaderProps {
  wowDir: WowDir;
}

/** The title, which log is showing (live or replay), and the replay control */
export function Header({ wowDir }: HeaderProps) {
  const status = useReplayStatus();
  const logs = useCombatLogs(wowDir.path); 

  return (
    <header className="header">
      <h1 className="header-title">Corvid</h1>

      {wowDir.path ? (
        <select
          className="select"
          aria-label="log"
          value={status?.log ?? ""}
          onFocus={logs.refresh}
          onChange={(e) => watch(e.target.value || null)}
        >
          <option value="">Live</option>
          {logs.names.map((name) => (
            <option key={name} value={name}>
              {logLabel(name)}
            </option>
          ))}
        </select>
      ) : (
        status && <span className="muted">{status.log}</span>
      )}

      {status && <ReplayControls status={status} />}

      {wowDir.path && (
        <button
          type="button"
          className="link header-folder"
          title={`Reading combat logs from ${wowDir.path}. Click to change`}
          onClick={wowDir.choose}
        >
          {folderName(wowDir.path)}
        </button>
      )}
    </header>
  );
}

function ReplayControls({ status }: { status: ReplayStatus }) {
  if (status.loading) return <span className="muted">Loading&hellip;</span>;

  return (
    <div className="replay" role="group" aria-label="Replay">
      <button
        type="button"
        className="button"
        title="Previous encounter"
        disabled={status.encounter === 0}
        onClick={replay.prevEncounter}
      >
        Prev
      </button>
      <button type="button" className="button" onClick={status.playing ? replay.pause : replay.play}>
        {status.playing ? "Pause" : "Play"}
      </button>
      <button
        type="button"
        className="button"
        title="Next encounter"
        disabled={status.encounter >= status.encounters}
        onClick={replay.nextEncounter}
      >
        Next
      </button>
      <span className="replay-progress">
        {status.encounters === 0 ? "No encounters" : `Encounter ${status.encounter || "-"} / ${status.encounters}`}
        <span className="muted"> &middot; {formatClock(status.time)}</span>
      </span>
      {REPLAY_SPEEDS.map((speed) => (
        <button
          key={speed}
          type="button"
          className="chip"
          aria-pressed={status.speed === speed}
          onClick={() => replay.setSpeed(speed)}
        >
          {speed}x
        </button>
      ))}
    </div>
  )
}