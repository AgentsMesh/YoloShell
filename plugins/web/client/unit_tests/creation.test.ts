import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/hubError";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { createShell, startAgent, type CreationOps } from "../server/creation";
import { machineFor } from "../server/machineHandles";

const HUB = crypto.randomUUID();

function fixture(): { body: Record<string, unknown>; machine: string; address: object } {
  process.env.YOLO_HUB_ID = HUB;
  process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-creation-test.sock";
  const machine = machineFor(HUB);
  return {
    machine,
    address: { hub_id: HUB, terminal_id: crypto.randomUUID() },
    body: { machine, cwd: "/tmp", name: "created", provider: "codex" },
  };
}

function fake(
  address: object,
  calls: { method: string; params: any; destination: string; timeout?: number }[],
) {
  const capability = crypto.randomUUID();
  const ops: CreationOps = {
    async request(method, params, destination, timeout) {
      calls.push({ method, params, destination, timeout });
      if (method === "create") return { creation: { address, creation_capability: capability } };
      if (method === "agent_start") return { start: { address, provider_id: "codex", binding: "awaiting" } };
      return {};
    },
  };
  return { ops, capability };
}

describe("web terminal creation", () => {
  test("creates, finalizes and returns only an opaque terminal address", async () => {
    const { body, address } = fixture();
    const calls: { method: string; params: any; destination: string }[] = [];
    const { ops, capability } = fake(address, calls);
    const result = (await createShell(body, ops)) as any;
    expect(calls.map((call) => call.method)).toEqual(["create", "finalize"]);
    expect(calls.map((call) => call.timeout)).toEqual([45_000, 20_000]);
    expect(calls[0].destination).toBe(HUB);
    expect(calls[0].params.request.cwd).toBe("/tmp");
    expect(calls[0].params.request.schema_version).toBe(1);
    expect(calls[1].params.creation.creation_capability).toBe(capability);
    expect(JSON.stringify(result)).not.toContain(capability);
    expect(result.terminal.terminal_id).toBe((address as any).terminal_id);
    expect(result.terminal.machine).not.toBe(HUB);
  });

  test("starts an agent through AgentStart without a creation capability", async () => {
    const { body, address } = fixture();
    const calls: { method: string; params: any; destination: string }[] = [];
    const { ops } = fake(address, calls);
    const result = (await startAgent(body, ops)) as any;
    expect(calls).toHaveLength(1);
    expect(calls[0].method).toBe("agent_start");
    expect(calls[0].params.request.provider_id).toBe("codex");
    expect(calls[0].params.request.schema_version).toBe(4);
    expect(calls[0].timeout).toBe(45_000);
    expect(result.provider).toBe("codex");
    expect(result.binding).toBe("awaiting");
  });

  test("rejects an unknown provider and a missing directory", async () => {
    const { body } = fixture();
    const ops: CreationOps = { request: async () => ({}) };
    await expect(startAgent({ ...body, provider: "nope" }, ops)).rejects.toThrow();
    await expect(createShell({ ...body, cwd: " " }, ops)).rejects.toThrow();
  });

  test("maps a peer without AgentStart to a clear machine message", async () => {
    const { body } = fixture();
    const ops: CreationOps = {
      request: async () => {
        throw new HubError(-32601, "Method not found", undefined);
      },
    };
    await expect(startAgent(body, ops)).rejects.toThrow("does not support starting agents");
  });

  test("reports a timed-out mutation without claiming it failed", async () => {
    const { body } = fixture();
    const ops: CreationOps = {
      request: async () => {
        throw new HubTimeoutError("Hub request timed out");
      },
    };
    await expect(createShell(body, ops)).rejects.toThrow("may still be starting");
  });

  test("maps an unsupported Hub error to a user-facing conflict", async () => {
    const { body } = fixture();
    const ops: CreationOps = {
      request: async () => {
        throw new HubError(-32000, "unsupported", { kind: "hub", code: "unsupported" });
      },
    };
    await expect(createShell(body, ops)).rejects.toThrow("does not support");
    await expect(startAgent(body, ops)).rejects.toThrow("provider is not available");
  });
});
