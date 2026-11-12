import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import { EVENTS_CHANNEL, toEvents, type CorvidEvent } from "./event";
import { eventStore } from "./store";

/** Where events come from: the Rust BE in the app, a recorded session in the browser */
interface FeedSource {
  start(): Promise<void>;
  stop(): void;
}

/** Live event batches from the Rust BE. */
class TauriSource implements FeedSource {
  private unlisten: (() => void) | null = null;
  private run = 0;

  async start(): Promise<void> {
    const run = ++this.run;
    const unlisten = await listen<unknown>(EVENTS_CHANNEL, (message) => {
      eventStore.push(toEvents(message.payload, EVENTS_CHANNEL));
    });
    if (run === this.run) this.unlisten = unlisten;
    else unlisten();
  }

  stop(): void {
    this.run += 1;
    this.unlisten?.();
    this.unlisten = null;
  }
}

export const REPLAY_SPEEDS =  [1, 4, 20] as const;
export type ReplaySpeed = (typeof REPLAY_SPEEDS)[number];

/** How often the replay sends a batch to the store */
const TICK_MS = 50;

export interface ReplayStatus {
  speed: ReplaySpeed;
  elapsed: number;
  duration: number;
  done: boolean;
}

/** The fixutre is loaded on demand, so the app never loads it at startup. */
async function loadSample(): Promise<CorvidEvent[]> {
  const { default: data } = await import("../../../fixtures/events/sample-session.json");
  return toEvents(data, "sample-session.json").sort((a, b) => a.ts - b.ts);
}

/**
 * Plays a recorded session into the store, so the UI can be built in a
 * browser without the game or the Rust BE.
 * 
 * TS are shifted so the session starts now, and the spacing between events
 * is kepy. Above 1x they run ahead of the wall clock, so views should
 * measure durations from event tiemstamps, not Date.now()
 */
class ReplaySource implements FeedSource {
  private events: CorvidEvent[] | null = null;
  private next = 0;
  private clock = 0;
  private offset = 0;
  private lastTick = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  private run = 0;
  private status: ReplayStatus = { speed: 4, elapsed: 0, duration: 0, done: false };
  private listeners = new Set<() => void>();

  async start(): Promise<void> {
    const run = ++this.run;
    this.events ??= await loadSample();
    if (run === this.run) this.restart();
  }
  
  stop(): void {
    this.run += 1;
    this.clearTimer();
  }

  /** Clears the feed and plays the session from start */
  restart = (): void => {
    const events = this.events;
    if (!events) return;
    this.clearTimer();
    eventStore.reset();
    this.next = 0;
    this.clock = 0;
    this.offset = events.length > 0 ? Date.now() - events[0].ts : 0;
    this.lastTick = performance.now();
    const duration = events.length > 0 ? (events[events.length - 1].ts - events[0].ts) / 1000 : 0;
    this.setStatus({ elapsed: 0, duration: Math.round(duration), done: false });
    this.timer = setInterval(this.tick, TICK_MS);
    this.tick();
  };

  setSpeed = (speed: ReplaySpeed): void => {
    this.setStatus({ speed });
  };

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getStatus = (): ReplayStatus => this.status;

  private tick = (): void => {
    const events = this.events;
    if (!events) return;
    const now = performance.now();
    this.clock += (now - this.lastTick) * this.status.speed;
    this.lastTick = now;

    const batch: CorvidEvent[] = [];
    while (this.next < events.length && events[this.next].ts - events[0].ts <= this.clock) {
      const event = events[this.next++];
      batch.push({ ...event, ts: event.ts + this.offset });
    }
    eventStore.push(batch);

    const done = this.next >= events.length;
    if (done) this.clearTimer();
    this.setStatus({ elapsed: Math.min(Math.floor(this.clock / 1000), this.status.duration), done });
  };

  private clearTimer(): void {
    if (this.timer !== null) clearInterval(this.timer);
    this.timer = null;
  }

  /** Publishes a new status object only when something changed (once a second) */
  private setStatus(patch: Partial<ReplayStatus>): void {
    const next = { ...this.status, ...patch };
    const keys = Object.keys(next) as (keyof ReplayStatus)[];
    if (keys.every((key) => next[key] === this.status[key])) return;
    this.status = next;
    for (const listener of this.listeners) listener();
  }
}

/** The browser replay, or null inside the app. */
export const replay = isTauri() ? null : new ReplaySource();

/** Starts the feed for this env and retursn a func that stops it. */
export function startFeed(): () => void {
  const source: FeedSource = replay ?? new TauriSource();
  eventStore.reset();
  source.start().catch((error: unknown) => {
    console.error("[corvid] the event feed failed to start", error);
  });
  return () => source.stop();
}

const subscribeNothing = () => () => {};
const getNull = () => null;

/** Replay speed and progress, or null inside the app. */
export function useReplayStatus(): ReplayStatus | null {
  return useSyncExternalStore(replay?.subscribe ?? subscribeNothing, replay?.getStatus ?? getNull);
}