import { describe, expect, test } from "bun:test";
import { machineCatalogue } from "../server/inventory";
import { entryMachine, hubFor, labelFor, machineFor } from "../server/machineHandles";

const ENTRY = crypto.randomUUID();
const PEER = crypto.randomUUID();
process.env.YOLO_HUB_ID = ENTRY;
process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-machine-handles-test.sock";

describe("opaque machine handles", () => {
  test("handles are stable per Hub and map back only on the server", () => {
    const first = machineFor(PEER);
    expect(machineFor(PEER)).toBe(first);
    expect(first).not.toBe(PEER);
    expect(hubFor(first)).toBe(PEER);
    expect(() => hubFor("not-a-handle")).toThrow();
  });

  test("labels identify the entry and configured peers", () => {
    expect(labelFor(ENTRY)).toBe("This machine");
    expect(labelFor(PEER, "build-box")).toBe("build-box");
    expect(labelFor(crypto.randomUUID())).toMatch(/^Machine \d+$/);
  });

  test("the catalogue replaces every address origin with a machine handle", () => {
    const terminalId = crypto.randomUUID();
    const agentId = crypto.randomUUID();
    const view = machineCatalogue({
      terminals: {
        terminals: [
          { address: { hub_id: PEER, terminal_id: terminalId }, runtime: { status: "running" } },
        ],
      },
      agents: {
        agents: [
          {
            cursor: {
              target: {
                address: { hub_id: PEER, agent_id: agentId },
                execution_id: crypto.randomUUID(),
              },
            },
            terminal: {
              address: { hub_id: PEER, terminal_id: terminalId },
              runtime_epoch: crypto.randomUUID(),
            },
          },
        ],
      },
      hubs: { hubs: [{ hub_id: PEER, reachable: true }] },
    }, new Map([[PEER, { create: true, agent_start: false }]])) as any;
    expect(view.machine).toBe(entryMachine());
    expect(view.terminals.terminals[0].address.machine).toBe(machineFor(PEER));
    expect(view.agents.agents[0].cursor.target.address.machine).toBe(machineFor(PEER));
    expect(view.agents.agents[0].terminal.address.machine).toBe(machineFor(PEER));
    expect(view.hubs.hubs[0].machine).toBe(machineFor(PEER));
    expect(view.hubs.hubs[0].label).toBeTruthy();
    expect(view.hubs.hubs[0].capabilities).toEqual({ create: true, agent_start: false });
  });
});
