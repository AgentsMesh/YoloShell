import { describe, expect, test } from "bun:test";
import { agentContext } from "../browser/model";
import { dispatch, resetStore } from "../browser/store";
import type { Agent } from "../browser/types";

function agent(hub: string): Agent {
  return {
    cursor: {
      target: { address: { machine: hub, agent_id: crypto.randomUUID() }, execution_id: crypto.randomUUID() },
      sequence: 1,
    },
    terminal: { address: { machine: hub, terminal_id: crypto.randomUUID() }, runtime_epoch: crypto.randomUUID() },
    provider: { kind: "claude" },
  };
}

function setInventory(inventory: unknown) {
  dispatch({
    type: "catalogue/replace",
    inventory: inventory as any,
    snapshot: undefined,
    emptyStreak: 0,
    at: 0,
  });
}

describe("agent machine context", () => {
  test("prefixes a federated row with its configured machine label", () => {
    resetStore();
    const peer = crypto.randomUUID();
    setInventory({
      hubs: { hubs: [{ machine: crypto.randomUUID(), reachable: true }, { machine: peer, reachable: true, label: "build-box" }] },
      terminals: { terminals: [] },
      agents: { agents: [] },
    });
    expect(agentContext(agent(peer))).toContain("build-box");
  });

  test("leaves the local or unlabelled row unchanged", () => {
    resetStore();
    const peer = crypto.randomUUID();
    setInventory({
      hubs: { hubs: [{ machine: peer, reachable: true }] },
      terminals: { terminals: [] },
      agents: { agents: [] },
    });
    expect(agentContext(agent(peer))).not.toContain("undefined");
  });
});
