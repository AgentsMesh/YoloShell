import { describe, expect, test } from "bun:test";
import { buildIndex, providerKey, terminalKey } from "../browser/catalogueIndex";
import type { Agent, Inventory, Terminal } from "../browser/types";

function terminal(machine: string, id: string): Terminal {
  return { address: { machine, terminal_id: id }, runtime: { status: "running" } };
}

function agent(id: string, machine: string, terminalId: string): Agent {
  return {
    cursor: { target: { address: { machine, agent_id: id }, execution_id: `exec-${id}` } },
    terminal: { address: { machine, terminal_id: terminalId }, runtime_epoch: "epoch" },
    lifecycle: { state: "running" },
  };
}

describe("catalogue index", () => {
  test("indexes terminals by address and by bare id", () => {
    const first = terminal("hub-a", "t1");
    const second = terminal("hub-b", "t1");
    const index = buildIndex({
      terminals: { terminals: [first, second] },
      agents: { agents: [] },
      hubs: { hubs: [] },
    } as Inventory);
    expect(index.terminalsByAddress.get(terminalKey(first.address))).toBe(first);
    expect(index.terminalsByAddress.get(terminalKey(second.address))).toBe(second);
    expect(index.terminalsById.get("t1")).toEqual([first, second]);
  });

  test("keeps the first agent for a duplicate agent id", () => {
    const first = agent("a1", "hub-a", "t1");
    const second = agent("a1", "hub-b", "t2");
    const index = buildIndex({
      terminals: { terminals: [terminal("hub-a", "t1")] },
      agents: { agents: [first, second] },
      hubs: { hubs: [] },
    } as Inventory);
    expect(index.agentsById.get("a1")).toBe(first);
    expect(index.agentsByTerminal.get(terminalKey({ machine: "hub-b", terminal_id: "t2" }))).toEqual([
      second,
    ]);
  });

  test("indexes provider facts per machine", () => {
    const index = buildIndex({
      terminals: { terminals: [] },
      agents: { agents: [] },
      hubs: {
        hubs: [
          {
            machine: "hub-a",
            providers: [
              {
                provider_id: "codex",
                features: { conversation: true, interaction: "terminal" },
              },
            ],
          },
        ],
      },
    } as Inventory);
    expect(index.providers.get(providerKey("hub-a", "codex"))?.features).toEqual({
      conversation: true,
      interaction: "terminal",
    });
    expect(index.providers.get(providerKey("hub-b", "codex"))).toBeUndefined();
  });
});
