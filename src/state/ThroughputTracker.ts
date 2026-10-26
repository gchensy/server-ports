import type { AppStore } from "./AppStore";

export interface Sample {
  /** Bytes per second sent to the server and received from it, over the last tick. */
  up: number;
  down: number;
}

const WINDOW = 60;
const TICK_MS = 1000;
export const NO_SAMPLES: readonly Sample[] = [];

/** Turns the byte counters of the mappings into the last minute of throughput. */
export class ThroughputTracker {
  private readonly rings = new Map<string, Sample[]>();
  private readonly previous = new Map<string, { up: number; down: number; at: number }>();
  private readonly listeners = new Set<() => void>();
  private version = 0;
  private timer: number | null = null;

  constructor(private readonly store: AppStore) {}

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  readonly getVersion = (): number => this.version;

  start(): void {
    this.timer ??= window.setInterval(() => this.tick(), TICK_MS);
  }

  stop(): void {
    if (this.timer !== null) window.clearInterval(this.timer);
    this.timer = null;
  }

  samples(forwardId: string): readonly Sample[] {
    return this.rings.get(forwardId) ?? NO_SAMPLES;
  }

  private tick(): void {
    const now = performance.now();
    const alive = new Set<string>();
    for (const runtime of Object.values(this.store.getState().runtimes)) {
      for (const forward of runtime.forwards) {
        alive.add(forward.id);
        const before = this.previous.get(forward.id);
        // The counters start from zero with every session, so a drop means "no news".
        const fresh = before && forward.bytesUp >= before.up && forward.bytesDown >= before.down;
        const seconds = before ? (now - before.at) / 1000 : 1;
        const sample: Sample = fresh
          ? {
              up: (forward.bytesUp - before.up) / seconds,
              down: (forward.bytesDown - before.down) / seconds,
            }
          : { up: 0, down: 0 };
        const ring = [...(this.rings.get(forward.id) ?? []), sample].slice(-WINDOW);
        this.rings.set(forward.id, ring);
        this.previous.set(forward.id, { up: forward.bytesUp, down: forward.bytesDown, at: now });
      }
    }
    for (const id of [...this.rings.keys()]) {
      if (!alive.has(id)) {
        this.rings.delete(id);
        this.previous.delete(id);
      }
    }
    this.version++;
    this.listeners.forEach((listener) => listener());
  }
}
