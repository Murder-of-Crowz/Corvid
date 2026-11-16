interface ChooseLogsDirProps {
  onChoose: () => void;
}

/** Shown in the app until the player picks the folder WoW writes combat logs to. */
export function ChooseLogsDir({ onChoose }: ChooseLogsDirProps) {
  return (
    <section className="choose-logs">
      <h2 className="choose-logs-title">Where does WoW save your combat logs?</h2>
      <p>
        Choose the folder that holds your <code>WoWCombatLog-&hellip;.txt</code> files. Corvid remembers it for next time.
      </p>
      <p className="muted">
        No files there yet? Type <code>/combatlog</code> in game to start logging.
      </p>
      <button type="button" className="button button-primary" onClick={onChoose}>
        Choose folder&hellip;
      </button>
    </section>
  );
}