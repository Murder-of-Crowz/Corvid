interface ChooseWowDirProps {
  onChoose: () => void;
}

/** Shown in the app until the player picks the folder WoW writes combat logs to. */
export function ChooseWowDir({ onChoose }: ChooseWowDirProps) {
  return (
    <section className="choose-logs">
      <h2 className="choose-logs-title">Where is your World of Warcraft installed?</h2>
      <p>
        Choose the <strong>World of Warcraft</strong> folder, the one with <code>_forever_</code> in it. Corvid reads your
        combat logs from it and draws its maps from the game's own files. It remembers the folder for next time.
      </p>
      <p className="muted">
        Corvid needs combat logging on. The LUA automatically sets it, make sure there is a <code>Corvid</code> folder
        in the <code>addOn</code> folder that has both <code>Corvid.lua</code> and <code>Corvid.toc</code> in it.
      </p>
      <button type="button" className="button button-primary" onClick={onChoose}>
        Choose folder&hellip;
      </button>
    </section>
  );
}