import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CreationIntentStore } from "../server/creationIntent";
import type { CreationTiming } from "../server/creationPolicy";
import { machineFor } from "../server/machineHandles";

export const HUB = crypto.randomUUID();
process.env.YOLO_HUB_ID = HUB;
process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-jobs-test.sock";
export const MACHINE = machineFor(HUB);
export const ADDRESS = { hub_id: HUB, terminal_id: crypto.randomUUID() };

export type Call = { method: string; params: any };
export type Handler = (method: string, params: any, calls: Call[]) => unknown;

export const TIMING: CreationTiming = {
  backoffMs: [0, 0, 0],
  maxAttempts: 2,
  maxIntents: 10,
  ttlMs: 60_000,
};

export function store(): CreationIntentStore {
  return new CreationIntentStore(mkdtempSync(join(tmpdir(), "yolo-web-jobs-")));
}

export function fake(handler: Handler): { ops: any; calls: Call[] } {
  const calls: Call[] = [];
  return {
    calls,
    ops: {
      async request(method: string, params: any) {
        calls.push({ method, params });
        return handler(method, params, calls);
      },
    },
  };
}

export function body(extra: Record<string, unknown> = {}): Record<string, unknown> {
  return { machine: MACHINE, cwd: "/tmp", ...extra };
}

export function key(): string {
  return crypto.randomUUID().replaceAll("-", "");
}

export async function until(check: () => boolean): Promise<void> {
  const deadline = Date.now() + 1_000;
  while (Date.now() < deadline) {
    if (check()) return;
    await Bun.sleep(5);
  }
  throw new Error("condition was not reached");
}

/** Create succeeds and finalize succeeds; other methods belong to the test. */
export function created(handler: Handler = () => undefined): { ops: any; calls: Call[] } {
  return fake((method, params) => {
    if (method === "create") {
      return { creation: { address: ADDRESS, creation_capability: crypto.randomUUID() } };
    }
    if (method === "finalize") return {};
    return handler(method, params, []);
  });
}
