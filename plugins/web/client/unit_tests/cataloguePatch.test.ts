import { describe, expect, test } from "bun:test";
import { machinePatch } from "../server/cataloguePatch";
import type { MachineView } from "../server/machineView";

function terminal(id: string, status = "running") {
  return { address: { machine: "m1", terminal_id: id }, runtime: { status } };
}

function agent(id: string, state = "running") {
  return {
    cursor: { target: { address: { machine: "m1", agent_id: id } } },
    lifecycle: { state },
  };
}

function view(
  revision: number,
  terminals: unknown[] = [terminal("t1")],
  agents: unknown[] = [agent("a1")],
  extra: Partial<MachineView> = {},
): MachineView {
  return {
    machine: "m1",
    label: "Machine",
    entry: true,
    reachable: true,
    freshness: "live",
    epoch: "epoch-1",
    revision,
    capabilities: { create: true, agent_start: true },
    terminals,
    agents,
    ...extra,
  };
}

describe("catalogue machine patches", () => {
  test("a revision-only poll is not a frame", () => {
    expect(machinePatch(view(1), view(2)).kind).toBe("none");
  });

  test("upserts new and changed rows and removes missing rows", () => {
    const previous = view(3, [terminal("t1"), terminal("t2")], [agent("a1"), agent("a2")]);
    const next = view(
      7,
      [terminal("t1", "exited"), terminal("t3")],
      [agent("a3")],
      { reachable: false, freshness: "stale" },
    );
    const decision = machinePatch(previous, next);
    if (decision.kind !== "patch") throw new Error(`unexpected ${decision.kind}`);
    expect(decision.patch.base_revision).toBe(3);
    expect(decision.patch.revision).toBe(7);
    expect(decision.patch.terminals.upsert.map((row: any) => row.address.terminal_id)).toEqual([
      "t1",
      "t3",
    ]);
    expect(decision.patch.terminals.remove).toEqual(["t2"]);
    expect(decision.patch.agents.upsert.map((row: any) => row.cursor.target.address.agent_id)).toEqual([
      "a3",
    ]);
    expect(decision.patch.agents.remove).toEqual(["a1", "a2"]);
    expect(decision.patch.hub.reachable).toBe(false);
    expect(decision.patch.hub.epoch).toBe("epoch-1");
    expect(decision.patch.hub.revision).toBe(7);
  });

  test("a summary-only change still produces a patch", () => {
    const decision = machinePatch(view(1), view(2, [terminal("t1")], [agent("a1")], {
      reachable: false,
      freshness: "unreachable",
    }));
    if (decision.kind !== "patch") throw new Error(`unexpected ${decision.kind}`);
    expect(decision.patch.terminals).toEqual({ upsert: [], remove: [] });
    expect(decision.patch.agents).toEqual({ upsert: [], remove: [] });
  });

  test("an epoch change or an unkeyed row forces a snapshot", () => {
    expect(machinePatch(view(1), view(2, [terminal("t1")], [agent("a1")], {
      epoch: "epoch-2",
    })).kind).toBe("snapshot");
    const unkeyed = { address: { machine: "m1" } };
    expect(machinePatch(view(1), view(2, [terminal("t1"), unkeyed])).kind).toBe("snapshot");
  });
});
