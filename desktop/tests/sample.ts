import { toEvents, type CorvidEvent } from "../src/data/event";

/**
 * The sample session from fixtures/events, or null when it hasn't been built.
 * It isn't committed, so tests that need it are skipped in CI. Build it with
 * scripts/make-sample-session.mjs (see the README).
 */
const found = import.meta.glob<unknown>("../../fixtures/events/sample-session.json", {
  eager: true,
  import: "default",
});
const data = Object.values(found)[0];

export const sampleEvents: CorvidEvent[] | null =
  data === undefined ? null : toEvents(data, "sample-session.json");
