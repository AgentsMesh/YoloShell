import { ApiError } from "./errors";
import { invalidateInventory } from "./inventory";
import { live, type CreationOps } from "./creation";
import { CreationIntentStore, type CreationIntent } from "./creationIntent";
import {
  DEFAULT_TIMING,
  publicState,
  recoverable,
  retryDelay,
  type CreationTiming,
  type PublicState,
} from "./creationPolicy";
import { fingerprint, idempotencyKey, prepare, spec, type Kind } from "./creationRequest";
import { runIntent } from "./creationRunner";

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class CreationCoordinator {
  private readonly running = new Set<string>();
  private readonly timers = new Map<string, ReturnType<typeof setTimeout>>();
  private opened = false;

  constructor(
    private readonly store = new CreationIntentStore(),
    private readonly ops: CreationOps = live,
    private readonly timing: CreationTiming = DEFAULT_TIMING,
  ) {}

  private ensure(): void {
    if (this.opened) return;
    this.opened = true;
    try {
      this.store.open();
    } catch (error) {
      console.error("creation journal unavailable:", message(error));
      return;
    }
    for (const intent of this.store.all()) {
      if (intent.state !== "dispatched") continue;
      try {
        this.store.update({
          ...intent,
          state: "unconfirmed",
          attempts: 0,
          error:
            "The plugin restarted before the Hub confirmed this creation. Checking the machine now.",
          updatedAt: Date.now(),
        });
      } catch (error) {
        console.error("creation journal normalization failed:", message(error));
      }
    }
  }

  start(kind: Kind, body: Record<string, unknown>): { job_id: string; state: PublicState } {
    this.ensure();
    this.store.clean(Date.now(), this.timing.ttlMs);
    const key = idempotencyKey(body);
    const expected = fingerprint(kind, body);
    const existing = this.store.byKey(key);
    if (existing) {
      if (existing.fingerprint !== expected) {
        throw new ApiError(409, "That retry key was already used for a different creation request");
      }
      if (this.retryAfterRestart(existing)) this.schedule(existing, 0);
      return { job_id: existing.id, state: publicState(existing) };
    }
    if (this.store.size() >= this.timing.maxIntents) {
      throw new ApiError(
        429,
        "Too many creation requests are awaiting reconciliation; check Agents and Shells before retrying",
      );
    }
    const prepared = prepare(spec(kind, body));
    const now = Date.now();
    const intent: CreationIntent = {
      version: 1,
      id: crypto.randomUUID(),
      idempotency_key: key,
      fingerprint: expected,
      kind,
      origin: prepared.origin,
      request: prepared.request,
      ...(prepared.provider ? { provider: prepared.provider } : {}),
      state: "prepared",
      attempts: 0,
      createdAt: now,
      updatedAt: now,
    };
    this.store.insert(intent);
    this.schedule(intent, 0);
    return { job_id: intent.id, state: "running" };
  }

  job(id: string): unknown {
    this.ensure();
    const intent = this.store.get(id);
    if (!intent) throw new ApiError(404, "Creation job was not found");
    return {
      job_id: intent.id,
      kind: intent.kind,
      state: publicState(intent),
      ...(intent.result !== undefined ? { result: intent.result } : {}),
      ...(intent.error ? { error: intent.error } : {}),
    };
  }

  /** Called once after startup: continue every intent the old process left open. */
  recover(): void {
    this.ensure();
    for (const intent of this.store.all()) {
      if (recoverable(intent)) this.schedule(intent, 0);
    }
  }

  stats(): { intents: number; unconfirmed: number } {
    this.ensure();
    return {
      intents: this.store.size(),
      unconfirmed: this.store.count("unconfirmed"),
    };
  }

  private retryAfterRestart(intent: CreationIntent): boolean {
    if (intent.state !== "unconfirmed") return intent.state !== "settled" && intent.state !== "failed";
    if (intent.attempts < this.timing.maxAttempts) return true;
    return Date.now() - intent.updatedAt > 60_000;
  }

  private schedule(intent: CreationIntent, delay: number): void {
    if (!recoverable(intent) || this.running.has(intent.id) || this.timers.has(intent.id)) return;
    const timer = setTimeout(() => {
      this.timers.delete(intent.id);
      void this.run(intent.id);
    }, delay);
    this.timers.set(intent.id, timer);
  }

  private async run(id: string): Promise<void> {
    if (this.running.has(id)) return;
    const intent = this.store.get(id);
    if (!intent || !recoverable(intent)) return;
    this.running.add(id);
    let persisted = false;
    try {
      const next = await runIntent(intent, this.ops, (value) => this.store.update(value));
      this.store.update(next);
      persisted = true;
      if (next.state === "settled") invalidateInventory();
    } catch (error) {
      console.error(`creation ${id} could not be recorded:`, message(error));
    } finally {
      this.running.delete(id);
    }
    const current = this.store.get(id);
    if (
      persisted &&
      current?.state === "unconfirmed" &&
      current.attempts < this.timing.maxAttempts
    ) {
      this.schedule(current, retryDelay(current.attempts, this.timing));
    }
  }
}

export const coordinator = new CreationCoordinator();

export function startJob(kind: Kind, body: Record<string, unknown>): unknown {
  return coordinator.start(kind, body);
}

export function getJob(id: string): unknown {
  return coordinator.job(id);
}

export function recoverCreations(): void {
  coordinator.recover();
}

export function creationStats(): { intents: number; unconfirmed: number } {
  return coordinator.stats();
}
