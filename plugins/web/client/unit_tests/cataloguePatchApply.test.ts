import { describe, expect, test } from "bun:test";
import { applyCataloguePatch, type CataloguePatch } from "../browser/cataloguePatch";
import type { Inventory } from "../browser/types";

function inventory(): Inventory {
  return {
    snapshot: { server: "s1", sequence: 10 },
    machine: "m1",
    hubs: {
      hubs: [
        {
          machine: "m1",
          label: "This machine",
          entry: true,
          reachable: true,
          freshness: "live",
          epoch: "e1",
          revision: 3,
          capabilities: { create: true, agent_start: true },
        },
      ],
    },
    terminals: {
      terminals: [
        { address: { machine: "m1", terminal_id: "t1" }, runtime: { status: "running" } },
        { address: { machine: "m1", terminal_id: "t2" }, runtime: { status: "running" } },
      ],
    },
    agents: {
      agents: [
        {
          cursor: { target: { address: { machine: "m1", agent_id: "a1" } } },
          lifecycle: { state: "running" },
        },
      ],
    },
  };
}

function patch(extra: Partial<CataloguePatch> = {}): CataloguePatch {
  return {
    machine: "m1",
    epoch: "e1",
    base_revision: 3,
    revision: 4,
    hub: {
      machine: "m1",
      label: "This machine",
      entry: true,
      reachable: true,
      freshness: "live",
      epoch: "e1",
      revision: 4,
      capabilities: { create: true, agent_start: true },
    },
    snapshot: { server: "s1", sequence: 11 },
    terminals: {
      upsert: [
        { address: { machine: "m1", terminal_id: "t3" }, runtime: { status: "running" } },
      ],
      remove: ["t2"],
    },
    agents: {
      upsert: [
        {
          cursor: { target: { address: { machine: "m1", agent_id: "a2" } } },
          lifecycle: { state: "starting" },
        },
      ],
      remove: ["a1"],
    },
    ...extra,
  };
}

describe("browser catalogue patch application", () => {
  test("applies row upserts/removals and advances the machine revision", () => {
    const result = applyCataloguePatch(inventory(), patch());
    if (!result.applied) throw new Error("patch was rejected");
    const terminals = result.inventory.terminals?.terminals ?? [];
    expect(terminals.map((row) => row.address?.terminal_id)).toEqual(["t1", "t3"]);
    expect(result.inventory.agents?.agents?.map((row) => row.cursor?.target?.address?.agent_id)).toEqual([
      "a2",
    ]);
    expect(result.inventory.hubs?.hubs?.[0]?.revision).toBe(4);
    expect(result.inventory.snapshot).toEqual({ server: "s1", sequence: 11 });
  });

  test("rejects a base revision or epoch gap", () => {
    expect(applyCataloguePatch(inventory(), patch({ base_revision: 2 })).applied).toBe(false);
    expect(applyCataloguePatch(inventory(), patch({ epoch: "other" })).applied).toBe(false);
    expect(
      applyCataloguePatch(
        { ...inventory(), hubs: { hubs: [] } },
        patch(),
      ).applied,
    ).toBe(false);
  });

  test("rejects a stale sequence, a server change and an invalid row key", () => {
    expect(
      applyCataloguePatch(inventory(), patch({ snapshot: { server: "s1", sequence: 10 } })).applied,
    ).toBe(false);
    expect(
      applyCataloguePatch(inventory(), patch({ snapshot: { server: "s2", sequence: 12 } })).applied,
    ).toBe(false);
    const bad = patch({
      terminals: {
        upsert: [{ address: { machine: "m1" }, runtime: { status: "running" } }],
        remove: [],
      },
    });
    expect(applyCataloguePatch(inventory(), bad).applied).toBe(false);
  });
});
