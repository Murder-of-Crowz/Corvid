import { useSyncExternalStore } from "react";
import type { CorvidEvent } from "./event";

const PUBLISH_INTERVAL_MS = 100;
const MAX_EVENTS = 5000;

export interface FeedSnapshot {
  events: readonly CorvidEvent[];
  firstSeq: number;
  total: number;
}

const EMPTY: FeedSnapshot = { events: [], firstSeq: 0, total: 0 };

/**
 * Holds the live event feed outside React. Batches can arrive many times a
 * second. Subscribers are notified at most every PUBLISH_INTERVAL_MS.
 */
class EventStore {
  private events: CorvidEvent[] = [];
  private total = 0;
  private snapshot = EMPTY;
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setTimeout> | null = null;

  push(batch: readonly CorvidEvent[]): void {
    if (batch.length === 0) return;
    for (const event of batch) this.events.push(event);
    this.total += batch.length;
    const excess = this.events.length - MAX_EVENTS;
    if (excess > 0) this.events.splice(0, excess);
    this.timer ??= setTimeout(() => this.publish(), PUBLISH_INTERVAL_MS);
  }

  /** Clears everything, i.e. when the replay restarts */
  reset(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.events = [];
    this.total = 0;
    this.snapshot = EMPTY;
    this.emit();
  }

  /** Arrow props so React can call them unbound */
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getSnapshot = (): FeedSnapshot => this.snapshot;

  private publish(): void {
    this.timer = null;
    this.snapshot = {
      events: this.events.slice(),
      firstSeq: this.total - this.events.length,
      total: this.total,
    };
    this.emit();
  }

  private emit(): void {
    for (const listener of this.listeners) listener();
  }
}

export const eventStore = new EventStore();

/** The current feed. Re-renders the component when a new snapshot is published */
export function useEventFeed(): FeedSnapshot {
  return useSyncExternalStore(eventStore.subscribe, eventStore.getSnapshot);
}