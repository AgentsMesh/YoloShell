import { HubClient, HubError } from "../../sdk/src/hub.ts";
import { HubTimeoutError } from "../../sdk/src/hubWire.ts";
import { ApiError } from "./errors";

export type LaneKind =
  | "query"
  | "conversation"
  | "creation"
  | "interaction"
  | "lifecycle"
  | "archive"
  | "admin";

type LaneConfig = { limit: number; queue: number };
type Lane = {
  config: LaneConfig;
  clients: (HubClient | null)[];
  active: number;
  waiting: number;
  waiters: ((slot: number) => void)[];
};

const LIMITS: Record<LaneKind, LaneConfig> = {
  query: { limit: 2, queue: 64 },
  conversation: { limit: 4, queue: 64 },
  creation: { limit: 2, queue: 8 },
  interaction: { limit: 2, queue: 16 },
  lifecycle: { limit: 1, queue: 8 },
  archive: { limit: 1, queue: 4 },
  admin: { limit: 1, queue: 8 },
};

const lanes = new Map<string, Lane>();
const metrics = { calls: 0, timeouts: 0, rejected: 0 };

function endpoint(): { socket: string; hubId: string } {
  const socket = process.env.YOLO_HUB_SOCKET;
  const hubId = process.env.YOLO_HUB_ID;
  if (!socket || !hubId) throw new Error("YOLO_HUB_SOCKET/YOLO_HUB_ID are unavailable");
  return { socket, hubId };
}

export function hubId(): string {
  return endpoint().hubId;
}

export function hubEndpoint(): { socket: string; hubId: string } {
  return endpoint();
}

export function poolStats(): { calls: number; timeouts: number; rejected: number } {
  return { ...metrics };
}

function lane(destination: string, kind: LaneKind): Lane {
  const key = `${destination}:${kind}`;
  let value = lanes.get(key);
  if (!value) {
    value = {
      config: LIMITS[kind],
      clients: Array.from({ length: LIMITS[kind].limit }, () => null),
      active: 0,
      waiting: 0,
      waiters: [],
    };
    lanes.set(key, value);
  }
  return value;
}

function acquire(target: Lane): Promise<number> {
  if (target.active < target.config.limit) {
    target.active += 1;
    return Promise.resolve(target.active - 1);
  }
  if (target.waiting >= target.config.queue) {
    metrics.rejected += 1;
    return Promise.reject(new ApiError(503, "Hub is busy; retry shortly"));
  }
  target.waiting += 1;
  return new Promise((resolve) => target.waiters.push(resolve));
}

function release(target: Lane, slot: number): void {
  target.active -= 1;
  const waiter = target.waiters.shift();
  if (waiter) {
    target.waiting -= 1;
    target.active += 1;
    waiter(slot);
  }
}

async function clientAt(target: Lane, slot: number): Promise<HubClient> {
  const existing = target.clients[slot];
  if (existing?.connected) return existing;
  existing?.close();
  const next = await HubClient.connect(endpoint());
  target.clients[slot] = next;
  return next;
}

function domain(error: unknown): boolean {
  return error instanceof HubError || error instanceof ApiError;
}

function resetClient(target: Lane, slot: number, client: HubClient): void {
  client.close();
  if (target.clients[slot] === client) target.clients[slot] = null;
}

async function use<T>(
  destination: string,
  kind: LaneKind,
  operation: (hub: HubClient) => Promise<T>,
  retry: boolean,
): Promise<T> {
  const target = lane(destination, kind);
  const slot = await acquire(target);
  metrics.calls += 1;
  let client: HubClient | null = null;
  try {
    client = await clientAt(target, slot);
    try {
      return await operation(client);
    } catch (error) {
      if (error instanceof HubTimeoutError) metrics.timeouts += 1;
      if (!domain(error) || error instanceof HubTimeoutError) resetClient(target, slot, client);
      if (!retry || (domain(error) && !(error instanceof HubTimeoutError))) throw error;
      client = await clientAt(target, slot);
      return await operation(client);
    }
  } finally {
    if (client) release(target, slot);
    else release(target, slot);
  }
}

/** Fast, retryable catalogue and layout reads. */
export function read<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "query", operation, true);
}

/** Long-poll conversation reads; one lane per destination. */
export function watch<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "conversation", operation, true);
}

/** Slow creation mutations never retry. */
export function create<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "creation", operation, false);
}

/** Prompt, response and lease mutations never retry. */
export function interact<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "interaction", operation, false);
}

/** Terminal lifecycle mutations never retry. */
export function lifecycle<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "lifecycle", operation, false);
}

/** Bounded archived history reads; a large reply must not starve queries. */
export function archive<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "archive", operation, true);
}

/** Infrequent administrative mutations. */
export function admin<T>(destination: string, operation: (hub: HubClient) => Promise<T>): Promise<T> {
  return use(destination, "admin", operation, false);
}
