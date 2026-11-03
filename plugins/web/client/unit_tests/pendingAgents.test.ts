import { describe, expect, test } from "bun:test";
import {
  addPendingAgent,
  prunePendingAgents,
  type PendingAgent,
} from "../browser/pendingAgents";

const MACHINE = crypto.randomUUID();
const TERMINAL = crypto.randomUUID();
const pending: PendingAgent = {
  machine: MACHINE,
  terminalId: TERMINAL,
  provider: "codex",
  name: "new-agent",
};
const terminal = {
  address: { machine: MACHINE, terminal_id: TERMINAL },
  runtime: { status: "running" },
};
const agent = {
  cursor: { target: { address: { machine: MACHINE, agent_id: crypto.randomUUID() } } },
  terminal: { address: { machine: MACHINE, terminal_id: TERMINAL }, runtime_epoch: crypto.randomUUID() },
  lifecycle: { state: "running" },
};

describe("pending agent rows", () => {
  test("deduplicates a starting agent and prunes it when its terminal ends", () => {
    const added = addPendingAgent([], pending);
    expect(added).toHaveLength(1);
    expect(added[0]).toMatchObject(pending);
    expect(addPendingAgent(added, pending)).toHaveLength(1);
    expect(prunePendingAgents(added, [terminal], [])).toHaveLength(1);
    expect(prunePendingAgents(added, [], [])).toHaveLength(1);
    expect(prunePendingAgents(added, [], [], Date.now() + 120_000)).toEqual([]);
  });

  test("drops the pending row once a real agent claims the terminal", () => {
    expect(prunePendingAgents([pending], [terminal], [agent as any])).toEqual([]);
  });

  test("drops a pending row when the catalogue reports its terminal ended", () => {
    expect(
      prunePendingAgents(
        [{ ...pending, at: Date.now() }],
        [{ ...terminal, runtime: { status: "exited" } }],
        [],
      ),
    ).toEqual([]);
  });
});
