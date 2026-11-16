import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import { EVENTS_CHANNEL, toEvents, type CorvidEvent, type PositionEvent } from "./event";
import { eventStore, MAX_EVENTS } from "./store";
import { type Heading, headingOf, isGroupPosition, latestPosition, nextHeading } from "./tracks";

/**
 * Live event batches from the Rust BE. They are kept while a replay is
 * showing, so going back to live loses nothing.
 */
class LiveSource {
  private events: CorvidEvent[] = [];
  private position: PositionEvent | null = null;
  private heading: Heading | null = null;
  private party = new Map<string, PositionEvent>();
  private partyList: readonly PositionEvent[] = [];
  private positionListeners = new Set<() => void>();
  private showing = false;
  private unlisten: (() => void) | null = null;
  private run = 0

  async start(): Promise<void> {
    const run = ++this.run;
    const unlisten = await listen<unknown>(EVENTS_CHANNEL, (message) => {
      const batch = toEvents(message.payload, EVENTS_CHANNEL);
      const position = latestPosition(batch);
      if (position) {
        this.position = position;
        this.heading = nextHeading(this.heading, position);
      }
      const members = batch.filter(isGroupPosition);
      for (const member of members) {
        if (member.mapId === 0) this.party.delete(member.unit.guid);
        else this.party.set(member.unit.guid, member);
      }
      if (members.length > 0) this.partyList = [...this.party.values()];
      if (position || members.length > 0) {
        for (const listener of this.positionListeners) listener();
      }
      const kept = batch.filter((event) => event.type !== "position" || event.source !== "strip");
      for (const event of kept) this.events.push(event);
      const excess = this.events.length - MAX_EVENTS;
      if (excess > 0) this.events.splice(0, excess);
      if (this.showing) eventStore.push(kept);
    });
    if (run === this.run) this.unlisten = unlisten;
    else unlisten();
  }
  
  stop(): void {
    this.run += 1;
    this.unlisten?.();
    this.unlisten = null;
  }

  /** Puts the live events back in the store, and sends new ones there as they come. */
  show(): void {
    this.showing = true;
    eventStore.reset(this.events);
  }

  hide(): void {
    this.showing = false;
  }

  getPosition = (): PositionEvent | null => this.position;

  getHeading = (): number | null => this.heading?.degrees ?? null;

  getParty = (): readonly PositionEvent[] => this.partyList;

  subscribePosition = (listener: () => void): (() => void) => {
    this.positionListeners.add(listener);
    return () => {
      this.positionListeners.delete(listener);
    }
  }
}

export const REPLAY_SPEEDS = [1, 4, 20] as const;
export type ReplaySpeed = (typeof REPLAY_SPEEDS)[number];

/** How often the replay sends a batch to the store */
const TICK_MS = 50;
/** Longer quiet stretches, like walking between fights, are cut down to this. */
const MAX_QUIET_MS = 5000;

export interface ReplayStatus {
  /** The log being replayed: a file name in the app, the sample in the browser. */
  log: string;
  loading: boolean;
  playing: boolean;
  speed: ReplaySpeed;
  encounter: number;
  encounters: number;
  time: number;
  done: boolean;
}

/** 
 * Plays a whole log into the store, with play, pause and jumps between
 * encounters. Events keep the log's timestamps, so views show the times 
 * the fights really happened, and should measure duration from them,
 * not Date.now().
 */
export class ReplaySource {
  private load: (log: string) => Promise<CorvidEvent[]>;
  private events: CorvidEvent[] = [];
  private starts: number[] = [];
  private next = 0;
  private clock = 0
  private lastTick = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  private run = 0
  private speed: ReplaySpeed = 1;
  private status: ReplayStatus | null = null;
  private listeners = new Set<() => void>();

  constructor(load: (log: string) => Promise<CorvidEvent[]>) {
    this.load = load;
  }

  /** Loads `log` and plays it from the start */
  async open(log: string): Promise<void> {
    const run = ++this.run;
    this.clearTimer();
    eventStore.reset();
    this.setStatus({ log, loading: true, playing: false, speed: this.speed, encounter: 0, encounters: 0, time: 0, done: false });

    const events = (await this.load(log)).sort((a, b) => a.ts - b.ts);
    if (run !== this.run) return;
    this.events = events;
    this.starts = events.filter((event) => event.type === "fightStart").map((event) => event.ts);
    this.seekStart();
    this.update({ loading: false });
    this.play();
  }

  /** Stops the replay, i.e. when going back to live. */
  close(): void {
    this.run += 1;
    this.clearTimer();
    this.setStatus(null);
  }

  /** Arrow props so React can call them unbound */
  play = (): void => {
    if (!this.status || this.status.loading) return;
    if (this.next >= this.events.length) this.seekStart();
    this.lastTick = performance.now();
    this.timer ??= setInterval(this.tick, TICK_MS);
    this.update({ playing: true });
  };

  pause = (): void => {
    this.clearTimer();
    this.update({ playing: false });
  };

  setSpeed = (speed: ReplaySpeed): void => {
    this.speed = speed;
    this.update({ speed });
  };

  /** Jumps to the start of the next encounter. */
  nextEncounter = (): void => {
    const start = this.starts[this.encountersStarted()];
    if (start !== undefined) this.seek(start);
  };

  /** Jumps to the start of the previous encounter, or from the first to the start of the log. */
  prevEncounter = (): void => {
    const current = this.encountersStarted();
    if (current >= 2) this.seek(this.starts[current - 2]);
    else this.seekStart();
  };

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getStatus = (): ReplayStatus | null => this.status;

  private tick = (): void => {
    const now = performance.now();
    this.clock += (now - this.lastTick) * this.speed;
    this.lastTick = now;
    const upcoming = this.events[this.next];
    if (upcoming && upcoming.ts - this.clock > MAX_QUIET_MS) this.clock = upcoming.ts - MAX_QUIET_MS;

    const first = this.next;
    while (this.next < this.events.length && this.events[this.next].ts <= this.clock) this.next++;
    eventStore.push(this.events.slice(first, this.next));
    if (this.next >= this.events.length) this.clearTimer();
    this.update({ ...this.progress(), playing: this.timer !== null });
  };

  private seek(ts: number): void {
    this.clock = ts;
    this.next = 0;
    while (this.next < this.events.length && this.events[this.next].ts <= ts) this.next++;
    eventStore.reset(this.events.slice(0, this.next));
    this.lastTick = performance.now();
    this.update(this.progress());
  }

  private seekStart(): void {
    this.seek((this.events[0]?.ts ?? 0) - 1);
  }

  private encountersStarted(): number {
    return this.starts.filter((ts) => ts <= this.clock).length;
  }

  private progress(): Pick<ReplayStatus, "encounter" | "encounters" | "time" | "done"> {
    return {
      encounter: this.encountersStarted(),
      encounters: this.starts.length,
      time: Math.floor(this.clock / 1000) * 1000,
      done: this.next >= this.events.length,
    };
  }

  private clearTimer(): void {
    if (this.timer !== null) clearInterval(this.timer);
    this.timer = null;
  }

  /** Publishes a new statuys object only when something changed (about once a second while playing) */
  private update(patch: Partial<ReplayStatus>): void {
    if (!this.status) return;
    const next = { ...this.status, ...patch };
    const keys = Object.keys(next) as (keyof ReplayStatus)[];
    if (keys.every((key) => next[key] === this.status?.[key])) return;
    this.setStatus(next);
  }

  private setStatus(status: ReplayStatus | null): void {
    this.status = status;
    for (const listener of this.listeners) listener();
  }
}

/**
 * The sample session is not committed. scripts/make-sample-session.mjs builds it.
 * import.meta.glob finds it when it exists and returns nothing when it doesn't,
 * so the app still builds without it. It's loaded on demand, never at startup.
 */
const samples = import.meta.glob<unknown>("../../../fixtures/events/sample-session.json", { import: "default" });

const SAMPLE = "Sample session";

/** A log's events: from the log folder in the app, the sample in the browser. */
async function loadLog(log: string): Promise<CorvidEvent[]> {
  if (isTauri()) return toEvents(await invoke<unknown>("read_log", { name: log }), log);
  const load = Object.values(samples)[0];
  if (!load) {
    console.warn("[corvid] No sample session. Run scripts/make-sample-session.mjs, then restart npm run dev.");
    return [];
  }
  return toEvents(await load(), "sample-session.json");
}

const live = isTauri() ? new LiveSource() : null;
export const replay = new ReplaySource(loadLog);

/** Starts the feed, live in the app or the sample replay in the browser, and returns a func that stops it. */
export function startFeed(): () => void {
  if (live) {
    live.start().catch(report);
    live.show();
  } else {
    watch(SAMPLE);
  }
  return () => {
    live?.stop();
    replay.close();
  };
}

/** Replays `log`, or goes back to live when it's null. */
export function watch(log: string | null): void {
  if (log === null) {
    replay.close();
    live?.show();
    return;
  }
  live?.hide();
  replay.open(log).catch((error: unknown) => {
    report(error);
    watch(null);
  });
}

/** The replay's status, or null while live. */
export function useReplayStatus(): ReplayStatus | null {
  return useSyncExternalStore(replay.subscribe, replay.getStatus);
}

/** A browser has no live feed, so there the live map follows the sample replay. */
const positions = live
  ? { subscribe: live.subscribePosition, get: live.getPosition }
  : { subscribe: eventStore.subscribe, get: () => latestPosition(eventStore.getSnapshot().events) };

/** Your newest live position, even while a replay is showing. Null until the first one. */
export function useLivePosition(): PositionEvent | null {
  return useSyncExternalStore(positions.subscribe, positions.get);
}

/** The same for whuich way you last moved. */
const headings = live
  ? { subscribe: live.subscribePosition, get: live.getHeading }
  : { subscribe: eventStore.subscribe, get: () => headingOf(eventStore.getSnapshot().events) };

/** Which way you last moved, in degrees clockwise from up. Null until you've moved on this map. */
export function useLiveHeading(): number | null {
  return useSyncExternalStore(headings.subscribe, headings.get);
}

const NO_PARTY: readonly PositionEvent[] = [];

/** The same for your party. The sample session has none of their positions. */
const partyPositions = live
  ? { subscribe: live.subscribePosition, get: live.getParty }
  : { subscribe: eventStore.subscribe, get: () => NO_PARTY };

/** Your party member's newest live positions, leaving out anyone who left your map or the group. */
export function useLiveParty(): readonly PositionEvent[] {
  return useSyncExternalStore(partyPositions.subscribe, partyPositions.get);
}

function report(error: unknown) {
  console.error("[corvid] the event feed failed", error);
}