import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/hubError";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { runLifecycle, type LifecycleOps } from "../server/lifecycle";
import { machineFor } from "../server/machineHandles";

const HUB = crypto.randomUUID();
process.env.YOLO_HUB_ID = HUB;
process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-lifecycle-test.sock";
const MACHINE = machineFor(HUB);
const TERMINAL = crypto.randomUUID();
const EPOCH = crypto.randomUUID();

function fake(reply: unknown = undefined) {
  const calls: { method: string; params: any; destination: string; timeout?: number }[] = [];
  const ops: LifecycleOps = {
    async request(method, params, destination, timeout) {
      calls.push({ method, params, destination, timeout });
      if (reply !== undefined) return reply;
      return {
        terminal: {
          address: { hub_id: HUB, terminal_id: TERMINAL },
          archived: false,
          runtime: { status: "exited" },
        },
      };
    },
  };
  return { ops, calls };
}

function body(extra: Record<string, unknown> = {}): Record<string, unknown> {
  return { machine: MACHINE, terminal_id: TERMINAL, runtime_epoch: EPOCH, action: "terminate", ...extra };
}

describe("web terminal lifecycle", () => {
  test("sends an exact terminate command and returns only opaque state", async () => {
    const { ops, calls } = fake();
    let changed = 0;
    const result = (await runLifecycle(body(), ops, () => (changed += 1))) as any;
    expect(calls).toHaveLength(1);
    expect(calls[0].method).toBe("terminal");
    expect(calls[0].destination).toBe(HUB);
    expect(calls[0].timeout).toBe(30_000);
    expect(calls[0].params.command.selection).toEqual({
      address: { hub_id: HUB, terminal_id: TERMINAL },
      expected_runtime: EPOCH,
    });
    expect(calls[0].params.command.action).toEqual({ operation: "terminate", force: false });
    expect(result.terminal).toEqual({
      machine: MACHINE,
      terminal_id: TERMINAL,
      archived: false,
      status: "exited",
    });
    expect(JSON.stringify(result)).not.toContain(HUB);
    expect(changed).toBe(1);
  });

  test("archive without an observed runtime sends a null fence", async () => {
    const { ops, calls } = fake({
      terminal: {
        address: { hub_id: HUB, terminal_id: TERMINAL },
        archived: true,
        runtime: { status: "exited" },
      },
    });
    const result = (await runLifecycle(
      body({ action: "archive", runtime_epoch: undefined }),
      ops,
      () => undefined,
    )) as any;
    expect(calls[0].params.command.selection.expected_runtime).toBeNull();
    expect(calls[0].params.command.action).toEqual({ operation: "archive" });
    expect(result.terminal.archived).toBe(true);
  });

  test("rejects an invalid action or terminal id before contacting the Hub", async () => {
    const { ops, calls } = fake();
    await expect(runLifecycle(body({ action: "delete" }), ops, () => undefined)).rejects.toThrow();
    await expect(runLifecycle(body({ terminal_id: "nope" }), ops, () => undefined)).rejects.toThrow();
    expect(calls).toHaveLength(0);
  });

  test("maps unknown outcomes and stale targets to actionable errors", async () => {
    const timeout: LifecycleOps = {
      async request() {
        throw new HubTimeoutError("timed out");
      },
    };
    await expect(runLifecycle(body(), timeout, () => undefined)).rejects.toThrow(
      "did not confirm the lifecycle change",
    );
    const unsupported: LifecycleOps = {
      async request() {
        throw new HubError(-32601, "Method not found", undefined);
      },
    };
    await expect(runLifecycle(body(), unsupported, () => undefined)).rejects.toThrow(
      "does not support ending",
    );
    const stale: LifecycleOps = {
      async request() {
        throw new HubError(-32000, "mismatch", { kind: "hub", code: "target_mismatch" });
      },
    };
    await expect(runLifecycle(body(), stale, () => undefined)).rejects.toThrow(
      "changed on the machine",
    );
  });
});
