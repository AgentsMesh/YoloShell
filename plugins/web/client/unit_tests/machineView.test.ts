import { describe, expect, test } from "bun:test";
import { machineView } from "../server/machineView";
import { machineFor } from "../server/machineHandles";

const HUB = crypto.randomUUID();
process.env.YOLO_HUB_ID = HUB;
process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-machine-view-test.sock";
describe("machine view privacy", () => {
  test("replaces every address origin without leaking a raw Hub id", () => {
    const terminal = crypto.randomUUID();
    const agent = crypto.randomUUID();
    const view = machineView({
      hub: HUB,
      entry: true,
      label: "build-box",
      reachable: true,
      freshness: "live",
      epoch: "epoch-1",
      revision: 1,
      methods: ["create", "agent_start", "terminal", "list_providers"],
      providers: [
        { provider_id: "codex", display_name: "Codex", available: true },
        { provider_id: "claude_code", display_name: "Claude Code", available: false },
      ],
      terminals: [
        {
          address: { hub_id: HUB, terminal_id: terminal },
          runtime: { target: { address: { hub_id: HUB, terminal_id: terminal } } },
        },
      ],
      agents: [
        {
          cursor: { target: { address: { hub_id: HUB, agent_id: agent } } },
          terminal: { address: { hub_id: HUB, terminal_id: terminal } },
        },
      ],
    });
    expect(view.machine).toBe(machineFor(HUB));
    expect(view.label).toBe("build-box");
    expect(view.epoch).toBe("epoch-1");
    expect(view.revision).toBe(1);
    expect(view.providers).toEqual([
      { provider_id: "codex", display_name: "Codex", available: true },
      { provider_id: "claude_code", display_name: "Claude Code", available: false },
    ]);
    expect(view.capabilities).toEqual({
      create: true,
      agent_start: true,
      terminal_lifecycle: true,
    });
    expect((view.terminals[0] as any).runtime.target.address.machine).toBe(machineFor(HUB));
    expect(JSON.stringify(view)).not.toContain(HUB);
    expect((view.agents[0] as any).cursor.target.address.machine).toBe(machineFor(HUB));
  });
});
