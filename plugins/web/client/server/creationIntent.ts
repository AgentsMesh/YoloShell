import { join } from "node:path";
import { readRecords, removeRecord, writeRecord } from "./creationJournal";
import type { Kind } from "./creationRequest";

export type IntentState = "prepared" | "dispatched" | "unconfirmed" | "settled" | "failed";

export type CreationIntent = {
  version: 1;
  id: string;
  idempotency_key: string;
  fingerprint: string;
  kind: Kind;
  /** Raw Hub id: server-side routing only, never returned to the browser. */
  origin: string;
  /** The complete original CreateShellRequest / AgentStartRequest. */
  request: Record<string, unknown>;
  provider?: string;
  state: IntentState;
  result?: unknown;
  error?: string;
  attempts: number;
  createdAt: number;
  updatedAt: number;
};

const TTL_MS = 10 * 60_000;

/**
 * One private journal record per creation intent. The record is written before
 * any Hub mutation, so a restart can reconcile instead of replaying Create.
 */
export class CreationIntentStore {
  private readonly intents = new Map<string, CreationIntent>();
  private readonly directory: string | null;
  private opened = false;

  constructor(data = process.env.YOLO_PLUGIN_DATA) {
    this.directory = data ? join(data, "creation-intents") : null;
  }

  open(): void {
    if (this.opened) return;
    this.opened = true;
    if (!this.directory) return;
    for (const intent of readRecords(this.directory)) {
      this.intents.set(intent.id, intent);
    }
  }

  all(): CreationIntent[] {
    this.open();
    return [...this.intents.values()];
  }

  get(id: string): CreationIntent | undefined {
    this.open();
    return this.intents.get(id);
  }

  byKey(key: string): CreationIntent | undefined {
    this.open();
    for (const intent of this.intents.values()) {
      if (intent.idempotency_key === key) return intent;
    }
    return undefined;
  }

  size(): number {
    this.open();
    return this.intents.size;
  }

  count(state: IntentState): number {
    this.open();
    let total = 0;
    for (const intent of this.intents.values()) {
      if (intent.state === state) total += 1;
    }
    return total;
  }

  insert(intent: CreationIntent): void {
    this.open();
    if (this.intents.has(intent.id)) throw new Error("duplicate creation intent");
    this.write(intent);
    this.intents.set(intent.id, intent);
  }

  update(intent: CreationIntent): void {
    this.open();
    if (!this.intents.has(intent.id)) throw new Error("unknown creation intent");
    this.write(intent);
    this.intents.set(intent.id, intent);
  }

  /** Settled/failed jobs are readable for ten minutes; uncertain evidence is kept. */
  clean(now = Date.now(), ttl = TTL_MS): number {
    this.open();
    let removed = 0;
    for (const intent of [...this.intents.values()]) {
      if (intent.state !== "settled" && intent.state !== "failed") continue;
      if (now - intent.updatedAt <= ttl) continue;
      this.remove(intent.id);
      removed += 1;
    }
    return removed;
  }

  private remove(id: string): void {
    this.intents.delete(id);
    if (this.directory) removeRecord(this.directory, id);
  }

  private write(intent: CreationIntent): void {
    if (this.directory) writeRecord(this.directory, intent);
  }
}
