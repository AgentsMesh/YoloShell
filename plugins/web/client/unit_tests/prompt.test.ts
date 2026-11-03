import { describe, expect, test } from "bun:test";
import { machineFor } from "../server/machineHandles";
import { prompt, type PromptOps } from "../server/write";

function fixture() {
  process.env.YOLO_HUB_ID = crypto.randomUUID();
  process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-prompt-test.sock";
  return {
    hub: crypto.randomUUID(),
    machine: "",
    agent_id: crypto.randomUUID(),
    execution_id: crypto.randomUUID(),
    terminal_id: crypto.randomUUID(),
    runtime_epoch: crypto.randomUUID(),
    text: "Run the tests",
  };
}

describe("web prompt", () => {
  test("sends the current schema to the agent's owning Hub", async () => {
    const body = fixture();
    body.machine = machineFor(body.hub);
    let request: any = null;
    let destination = "";
    const ops: PromptOps = {
      send: async (value, origin) => {
        request = value;
        destination = origin;
        return { prompt: { outcome: "delivered" } };
      },
    };
    const result = await prompt(body, ops);
    expect(result).toEqual({ prompt: { outcome: "delivered" } });
    expect(request.schema_version).toBe(4);
    expect(destination).toBe(body.hub);
    expect(request.reference.target.address.hub_id).toBe(body.hub);
    expect(request.prompt).toBe(body.text);
    expect(request.take_control).toBe(true);
  });

  test("releases the same tab's terminal control before prompting", async () => {
    const base = fixture();
    base.machine = machineFor(base.hub);
    const body = { ...base, client_id: "tab-1" };
    const order: string[] = [];
    let released = "";
    const ops: PromptOps = {
      release: async (_body, client) => {
        released = client;
        order.push("release");
      },
      send: async () => {
        order.push("send");
        return {};
      },
    };
    await prompt(body, ops);
    expect(released).toBe("tab-1");
    expect(order).toEqual(["release", "send"]);
  });

  test("retries a same-tab handoff after an explicit lease conflict", async () => {
    const base = fixture();
    base.machine = machineFor(base.hub);
    const body = { ...base, client_id: "tab-2" };
    let sends = 0;
    let releases = 0;
    const ops: PromptOps = {
      release: async () => {
        releases += 1;
      },
      send: async () => {
        sends += 1;
        if (sends === 1) {
          throw Object.assign(new Error("conflict"), { data: { code: "conflict" } });
        }
        return { prompt: { outcome: "delivered" } };
      },
    };
    expect(await prompt(body, ops)).toEqual({ prompt: { outcome: "delivered" } });
    expect(sends).toBe(2);
    expect(releases).toBeGreaterThanOrEqual(2);
  });

  test("defaults a missing origin to the entry Hub and validates the prompt", async () => {
    const body = fixture();
    let destination = "";
    const ops: PromptOps = {
      send: async (_request, origin) => {
        destination = origin;
        return {};
      },
    };
    const { machine: _ignored, ...withoutOrigin } = body;
    await prompt(withoutOrigin, ops);
    expect(destination).toBe(process.env.YOLO_HUB_ID);
    await expect(prompt({ ...withoutOrigin, text: " " }, ops)).rejects.toThrow();
  });
});
