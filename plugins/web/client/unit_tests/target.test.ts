import { describe, expect, test } from "bun:test";
import { machineFor } from "../server/machineHandles";
import { terminalTarget } from "../server/target";

const ENTRY = crypto.randomUUID();

function fixture() {
  process.env.YOLO_HUB_ID = ENTRY;
  process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-target-test.sock";
  const hub = crypto.randomUUID();
  return {
    hub,
    machine: machineFor(hub),
    terminal_id: crypto.randomUUID(),
    runtime_epoch: crypto.randomUUID(),
  };
}

describe("web terminal target", () => {
  test("resolves an opaque machine to its peer origin", () => {
    const body = fixture();
    expect(terminalTarget(body).address.hub_id).toBe(body.hub);
    const { machine: _ignored, hub: _hub, ...withoutMachine } = body;
    expect(terminalTarget(withoutMachine).address.hub_id).toBe(ENTRY);
  });

  test("rejects an invalid machine handle", () => {
    const body = fixture();
    expect(() => terminalTarget({ ...body, machine: "not-a-machine" })).toThrow();
  });
});
