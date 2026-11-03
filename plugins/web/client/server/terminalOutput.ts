import type { ServerWebSocket } from "bun";
import type { Seed } from "./terminalSeed";
import type { TerminalOutputFrame } from "../shared/protocol";

const CHUNK = 64 * 1024;
const HIGH_WATER = 4 * 1024 * 1024;
const RING_LIMIT = 8 * 1024 * 1024;

type Sink = { ws: ServerWebSocket<unknown> };

function bytes(ws: ServerWebSocket<unknown>, data: Uint8Array): boolean {
  for (let offset = 0; offset < data.byteLength; offset += CHUNK) {
    if (((ws as unknown as { getBufferedAmount?: () => number }).getBufferedAmount?.() ?? 0) > HIGH_WATER) {
      ws.close(1013, "subscriber is too slow");
      return false;
    }
    ws.send(data.subarray(offset, Math.min(offset + CHUNK, data.byteLength)));
  }
  return true;
}

function json(ws: ServerWebSocket<unknown>, value: TerminalOutputFrame): void {
  if (ws.readyState === 1) ws.send(JSON.stringify(value));
}

/** One actor fan-out: the seed and post-seed ring are shared by every tab. */
export class TerminalOutput {
  private readonly sinks = new Set<Sink>();
  private seed: Seed | null = null;
  private ring: Uint8Array[] = [];
  private ringBytes = 0;
  private ringValid = true;

  get count(): number {
    return this.sinks.size;
  }

  get needsSeed(): boolean {
    return this.seed === null || !this.ringValid;
  }

  /** The shared seed plus post-seed output, capped to the most recent bytes. */
  history(limit: number): { data: Uint8Array; truncated: boolean } | null {
    const seed = this.seed;
    if (!seed) return null;
    const chunks = [seed.data, ...this.ring];
    const total = chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
    const size = Math.min(total, limit);
    const data = new Uint8Array(size);
    let skip = total - size;
    let wrote = 0;
    for (const chunk of chunks) {
      if (skip >= chunk.byteLength) {
        skip -= chunk.byteLength;
        continue;
      }
      const source = chunk.subarray(skip);
      const take = Math.min(source.byteLength, size - wrote);
      data.set(source.subarray(0, take), wrote);
      wrote += take;
      skip = 0;
      if (wrote === size) break;
    }
    return { data, truncated: !this.ringValid || total > limit };
  }

  subscribe(ws: ServerWebSocket<unknown>): void {
    const sink = { ws };
    this.sinks.add(sink);
    if (this.seed && this.ringValid) this.sendSeed(sink, false);
  }

  unsubscribe(ws: ServerWebSocket<unknown>): void {
    for (const sink of this.sinks) if (sink.ws === ws) this.sinks.delete(sink);
  }

  invalidate(): void {
    this.seed = null;
    this.ring = [];
    this.ringBytes = 0;
    this.ringValid = true;
  }

  /** Keep the shared seed sized like the PTY the controller just resized. */
  resize(cols: number, rows: number): void {
    if (this.seed) this.seed = { ...this.seed, cols, rows };
  }

  install(seed: Seed): void {
    this.seed = seed;
    this.ring = [];
    this.ringBytes = 0;
    this.ringValid = true;
    for (const sink of this.sinks) this.sendSeed(sink, true);
  }

  reset(): void {
    for (const sink of this.sinks) json(sink.ws, { type: "reset" });
  }

  publish(data: Uint8Array): void {
    for (const sink of [...this.sinks]) {
      if (!bytes(sink.ws, data)) this.sinks.delete(sink);
    }
    if (!this.ringValid) return;
    if (this.ringBytes + data.byteLength > RING_LIMIT) {
      this.ring = [];
      this.ringBytes = 0;
      this.ringValid = false;
      return;
    }
    this.ring.push(Uint8Array.from(data));
    this.ringBytes += data.byteLength;
  }

  ended(code: number | null, signal: number | null): void {
    for (const sink of this.sinks) json(sink.ws, { type: "exit", code, signal });
  }

  unreachable(): void {
    for (const sink of this.sinks) json(sink.ws, { type: "unreachable" });
  }

  error(message: string): void {
    for (const sink of this.sinks) json(sink.ws, { type: "error", message });
  }

  private sendSeed(sink: Sink, reset: boolean): void {
    const seed = this.seed;
    if (!seed) return;
    if (reset) json(sink.ws, { type: "reset" });
    json(sink.ws, {
      type: "ready",
      cols: seed.cols,
      rows: seed.rows,
      seed: seed.kind,
    });
    if (!bytes(sink.ws, seed.data)) {
      this.sinks.delete(sink);
      return;
    }
    for (const chunk of this.ring) {
      if (!bytes(sink.ws, chunk)) {
        this.sinks.delete(sink);
        return;
      }
    }
    json(sink.ws, { type: "live" });
  }
}
