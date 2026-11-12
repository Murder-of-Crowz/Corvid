import { memo, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { CorvidEvent } from "../data/event";
import { useEventFeed } from "../data/store";
import { describe, formatClock } from "../lib/format";

const CATEGORIES = ["damage", "heal", "death", "fight", "position", "raw"] as const;
type Category = (typeof CATEGORIES)[number];

const LABELS: Record<Category, string> = {
  damage: "Damage",
  heal: "Heals",
  death: "Deaths",
  fight: "Fights",
  position: "Positions",
  raw: "Raw lines",
};

/** Only the newest matching rows are rendered, so the list stays fast. */
const MAX_ROWS = 500;
/** Scrolled to within this many px of the bottom counts as following */
const FOLLOW_SLACK_PX = 24;

function categoryOf(event: CorvidEvent): Category | null {
  switch (event.type) {
    case "combat":
      return event.kind;
    case "fightStart":
    case "fightEnd":
      return "fight";
    case "position":
      return "position";
    case "rawLine":
      return "raw";
    case "chat":
      return null // Waiting until chat is implemented
  }
}

interface Row {
  seq: number;
  event: CorvidEvent;
}

/** Every event as it arrives, newest at the bottom. A debugging view for the feed. */
export function EventsView() {
  const { events, firstSeq } = useEventFeed();
  const [hidden, setHidden] = useState<ReadonlySet<Category>>(() => new Set());
  const [following, setFollowing] = useState(true);
  const listRef = useRef<HTMLDivElement>(null);

  const counts = useMemo(() => {
    const counts = { damage: 0, heal: 0, death: 0, fight: 0, position: 0, raw: 0 };
    for (const event of events) {
      const category = categoryOf(event);
      if (category) counts[category] += 1
    }
    return counts;
  }, [events]);

  const rows = useMemo(() => {
    const rows: Row[] = [];
    for (let i = events.length - 1; i >= 0 && rows.length < MAX_ROWS; i--) {
      const category = categoryOf(events[i]);
      if (category && !hidden.has(category)) rows.push({ seq: firstSeq + i, event: events[i] });
    }
    return rows.reverse();
  }, [events, firstSeq, hidden])

  // Runs before the browser, new rows never flash below the fold.
  useLayoutEffect(() => {
    const list = listRef.current;
    if (list && following) list.scrollTop = list.scrollHeight;
  }, [rows, following]);

  function onScroll() {
    const list = listRef.current;
    if (!list) return;
    const atBottom = list.scrollHeight - list.scrollTop - list.clientHeight <= FOLLOW_SLACK_PX;
    if (atBottom !== following) setFollowing(atBottom);
  }

  function toggle(category:Category) {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(category)) next.delete(category);
      else next.add(category);
      return next;
    });
  }

  return (
    <section className="events">
      <div className="events-filters" role="group" aria-label="Event types">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            className="chip"
            aria-pressed={!hidden.has(cat)}
            onClick={() => toggle(cat)}
          >
            {LABELS[cat]} <span className="chip-count">{counts[cat]}</span> 
          </button>
        ))}
      </div>

      <div className="events-list" ref={listRef} onScroll={onScroll}>
        {rows.length > 0 ? (
          rows.map((row) => <EventRow key={row.seq} event={row.event} />)
        ) : (
          <p className="events-empty">
            {events.length > 0 ? "No events match these filters" : "Waiting for events..."}
          </p>
        )}
      </div>

      {!following && (
        <button type="button" className="jump-latest" onClick={() => setFollowing(true)}>
          Jump to latest
        </button>
      )}
    </section>
  );
}

const EventRow = memo(function EventRow({ event }: { event: CorvidEvent }) {
  return (
    <div className={`event-row event-${categoryOf(event) ?? "other"}`}>
      <time className="event-time" dateTime={new Date(event.ts).toISOString()}>
        {formatClock(event.ts)}
      </time>
      <span className="event-text">{describe(event)}</span>
    </div>
  )
})