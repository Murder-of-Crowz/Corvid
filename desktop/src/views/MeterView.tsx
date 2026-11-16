import { useMemo, useState } from "react";
import { buildFights, fightDuration, meterRows, type Fight, type MeterMode, type MeterRow } from "../data/fights";
import { useEventFeed } from "../data/store";
import { formatClock, formatDuration, formatNumber, formatPercent, formatRate } from "../lib/format";

const MODES: { id: MeterMode; label: string }[] = [
  { id: "damage", label: "Damage" },
  { id: "healing", label: "Healing" },
];

/** Dmg or healing per player, for the latesy fight or one picked from session. */
export function MeterView() {
  const { events } = useEventFeed();
  const [mode, setMode] = useState<MeterMode>("damage");
  const [pinnedId, setPinnedId] = useState<string | null>(null);
  const [openGuid, setOpenGuid] = useState<string | null>(null);

  const fights = useMemo(() => buildFights(events), [events]);
  const fight = fights.find((f) => f.id === pinnedId) ?? fights[fights.length - 1];
  const rows = useMemo(() => (fight ? meterRows(fight, mode) : []), [fight, mode]);

  if (!fight) {
    return <p className="view-empty">No fights yet. Damage and healing by you and your party show up here.</p>;
  }

  const top = rows[0]?.total ?? 0;

  return (
    <section className={`meter meter-${mode}`}>
      <select
        className="select meter-fight"
        aria-label="Fight"
        value={pinnedId ?? ""}
        onChange={(e) => setPinnedId(e.target.value || null)}
      >
        <option value="">Latest fight</option>
        {[...fights].reverse().map((f) => (
          <option key={f.id} value={f.id}>
            {fightLabel(f)}
          </option>
        ))}
      </select>

      <div className="meter-modes" role="group" aria-label="Show">
        {MODES.map((m) => (
          <button
            key={m.id}
            type="button"
            className="chip"
            aria-pressed={mode === m.id}
            onClick={() => setMode(m.id)}
          >
            {m.label}
          </button>
        ))}
      </div>

      <p className="meter-title">
        {fight.name} : <span className="muted">{fight.end === undefined ? "in combat" : formatClock(fight.end)}</span>
      </p>

      {rows.length === 0 ? (
        <p className="view-empty">No {mode} by your side in this fight.</p>
      ) : (
        <ol className="meter-rows">
          {rows.map((row) => (
            <MeterBar
              key={row.unit.guid}
              row={row}
              share={top > 0 ? row.total / top : 0}
              open={openGuid === row.unit.guid}
              onToggle={() => setOpenGuid(openGuid === row.unit.guid ? null : row.unit.guid)}
            />
          ))}
        </ol>
      )}
    </section>
  );
}

/** "09:55:11 Scarlet Covert ():13)" for the fight picker */
function fightLabel(fight: Fight): string {
  const status = fight.end === undefined ? ", in combat" : "";
  return `${formatClock(fight.start)}  ${fight.name} (${formatDuration(fightDuration(fight))}${status})`;
}

interface MeterBarProps {
  row: MeterRow;
  share: number;
  open: boolean;
  onToggle: () => void;
}

function MeterBar({ row, share, open, onToggle }: MeterBarProps) {
  return (
    <li>
      <button type="button" className="meter-bar" aria-expanded={open} onClick={onToggle}>
        <span className="meter-fill" style={{ width: `${share * 100}%` }} />
        <span className="meter-name">{row.unit.name}</span>
        <span className="meter-value">
          {formatNumber(row.total)} <span className="muted">{formatRate(row.perSecond)}/s</span>
        </span>
      </button>
      {open && <SpellTable row={row} />}
    </li>
  )
}

/** Per-spell breakdown for one player */
function SpellTable({ row }: { row: MeterRow }) {
  return (
    <table className="spells">
      <thead>
        <tr>
          <th scope="col">Spell</th>
          <th scope="col">Total</th>
          <th scope="col">Share</th>
          <th scope="col">Hits</th>
          <th scope="col">Crit</th>
        </tr>
      </thead>
      <tbody>
        {row.spells.map((spell) => (
          <tr key={spell.key}>
            <th scope="row">{spell.name}</th>
            <td>{formatNumber(spell.total)}</td>
            <td>{formatPercent(row.total > 0 ? spell.total / row.total : 0)}</td>
            <td>{spell.hits}</td>
            <td>{formatPercent(spell.crits / spell.hits)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}