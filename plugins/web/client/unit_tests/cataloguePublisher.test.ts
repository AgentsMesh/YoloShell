import { describe, expect, test } from "bun:test";
import { CataloguePublisher } from "../server/cataloguePublisher";
import type { MachineView } from "../server/machineView";

function terminal(id: string, status = "running") {
  return { address: { machine: "m1", terminal_id: id }, runtime: { status } };
}

function view(
  machine: string,
  revision: number,
  terminals: unknown[] = [terminal("t1")],
  extra: Partial<MachineView> = {},
): MachineView {
  return {
    machine,
    label: machine,
    entry: machine === "m1",
    reachable: true,
    freshness: "live",
    epoch: `epoch-${machine}`,
    revision,
    capabilities: { create: true, agent_start: true },
    terminals,
    agents: [],
    ...extra,
  };
}

describe("catalogue publisher", () => {
  test("the first observation is a snapshot with per-machine metadata", () => {
    const publisher = new CataloguePublisher("m1");
    const event = publisher.publish([view("m1", 1)], view("m1", 1));
    expect(event?.kind).toBe("snapshot");
    const inventory = (event as any).inventory;
    expect(inventory.snapshot.sequence).toBe(1);
    expect(inventory.machine).toBe("m1");
    expect(inventory.hubs.hubs[0]).toMatchObject({
      machine: "m1",
      epoch: "epoch-m1",
      revision: 1,
    });
    expect(publisher.latest()).toEqual(inventory);
  });

  test("a semantic change publishes a patch and advances the aggregate sequence", () => {
    const publisher = new CataloguePublisher("m1");
    publisher.publish([view("m1", 1)], view("m1", 1));
    const next = view("m1", 2, [terminal("t1"), terminal("t2")]);
    const event = publisher.publish([next], next);
    expect(event?.kind).toBe("patch");
    const patch = (event as any).patch;
    expect(patch).toMatchObject({
      machine: "m1",
      epoch: "epoch-m1",
      base_revision: 1,
      revision: 2,
      snapshot: { sequence: 2 },
    });
    expect(patch.terminals.upsert).toHaveLength(1);
    expect((publisher.latest() as any).terminals.terminals).toHaveLength(2);
  });

  test("a revision-only poll publishes nothing and keeps the base revision", () => {
    const publisher = new CataloguePublisher("m1");
    publisher.publish([view("m1", 1)], view("m1", 1));
    const quiet = view("m1", 2);
    expect(publisher.publish([quiet], quiet)).toBeNull();
    expect((publisher.latest() as any).snapshot.sequence).toBe(1);
    const changed = view("m1", 3, [terminal("t1"), terminal("t3")]);
    const event = publisher.publish([changed], changed);
    expect((event as any).patch.base_revision).toBe(1);
    expect((event as any).patch.revision).toBe(3);
  });

  test("a discovered machine forces a snapshot with both machines", () => {
    const publisher = new CataloguePublisher("m1");
    publisher.publish([view("m1", 1)], view("m1", 1));
    const second = view("m2", 1, [terminal("t2")], { entry: false });
    const event = publisher.publish([view("m1", 1), second], second);
    expect(event?.kind).toBe("snapshot");
    expect((event as any).inventory.hubs.hubs.map((hub: any) => hub.machine)).toEqual([
      "m1",
      "m2",
    ]);
  });

  test("gap repair and idle reset never move the aggregate sequence backwards", () => {
    const publisher = new CataloguePublisher("m1");
    publisher.publish([view("m1", 1)], view("m1", 1));
    const resync = publisher.forceSnapshot();
    expect((resync as any).inventory.snapshot.sequence).toBe(2);
    publisher.reset();
    expect(publisher.latest()).toBeNull();
    const next = publisher.publish([view("m1", 1)], view("m1", 1));
    expect((next as any).inventory.snapshot.sequence).toBe(3);
  });
});
