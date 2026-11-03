import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/hub.ts";
import { machineFor } from "../server/machineHandles";
import { listArchive, readArchivedHistory, type ArchiveOps } from "../server/archive";

function archived(hub: string, terminalId: string, epoch: string): Record<string, unknown> {
  return {
    address: { hub_id: hub, terminal_id: terminalId },
    name: "old-shell",
    cwd: "/tmp/old",
    archived: true,
    runtime: { status: "exited", target: { runtime_epoch: epoch } },
  };
}

function harness(rows: unknown[] = []) {
  const hub = crypto.randomUUID();
  const peer = crypto.randomUUID();
  const machine = machineFor(hub);
  const terminalId = crypto.randomUUID();
  const epoch = crypto.randomUUID();
  const calls: string[] = [];
  let request: Record<string, unknown> | null = null;
  const ops: ArchiveOps = {
    machines: () => [
      { hub, machine, label: "This machine", reachable: true },
      { hub: peer, machine: machineFor(peer), label: "Peer", reachable: false },
    ],
    list: async (destination) => {
      calls.push(destination);
      return { terminals: rows.length ? rows : [archived(hub, terminalId, epoch)] };
    },
    history: async (destination, body) => {
      calls.push(destination);
      request = body;
      return {
        history: {
          data: Buffer.from("hello").toString("base64"),
          from_sequence: 1,
          next_sequence: 2,
          truncated: false,
        },
      };
    },
  };
  return { hub, peer, machine, terminalId, epoch, calls, ops, request: () => request };
}

describe("web archive listing", () => {
  test("returns archived rows with opaque machine handles only", async () => {
    const { hub, machine, ops, calls } = harness();
    const result = (await listArchive(undefined, ops)) as {
      machines: { machine: string; label: string }[];
      terminals: Record<string, any>[];
      reachable: number;
    };
    expect(result.reachable).toBe(1);
    expect(result.machines).toEqual([
      { machine, label: "This machine", reachable: true },
      { machine: machineFor(ops.machines()[1].hub), label: "Peer", reachable: false },
    ]);
    expect(result.terminals).toHaveLength(1);
    expect(result.terminals[0].address.machine).toBe(machine);
    expect(JSON.stringify(result)).not.toContain(hub);
    expect(calls).toEqual([hub]);
  });

  test("hides archives from Hubs without the retained-history method", async () => {
    const { machine, ops } = harness();
    ops.list = async () => ({ terminals: [], supports: false });
    const result = (await listArchive(undefined, ops)) as {
      terminals: unknown[];
      unsupported: { machine: string; label: string }[];
    };
    expect(result.terminals).toHaveLength(0);
    expect(result.unsupported).toEqual([{ machine, label: "This machine" }]);
  });

  test("caps an unbounded archive list", async () => {
    const { hub, epoch, ops } = harness();
    const rows = Array.from({ length: 501 }, (_, index) =>
      archived(hub, crypto.randomUUID(), `${epoch}-${index}`),
    );
    ops.list = async () => ({ terminals: rows, supports: true });
    const result = (await listArchive(undefined, ops)) as {
      terminals: unknown[];
      truncated: boolean;
    };
    expect(result.terminals).toHaveLength(500);
    expect(result.truncated).toBe(true);
  });

  test("includes ended unarchived rows from the live catalogue", async () => {
    const { hub, machine, epoch, ops } = harness();
    const terminalId = crypto.randomUUID();
    ops.latest = () => ({
      terminals: {
        terminals: [
          {
            address: { machine, terminal_id: terminalId },
            name: "ended-shell",
            runtime: { status: "exited", target: { runtime_epoch: epoch } },
          },
        ],
      },
    });
    const result = (await listArchive(undefined, ops)) as {
      archived: number;
      finished: number;
      terminals: { finished?: boolean; address: { terminal_id: string } }[];
    };
    expect(result.archived).toBe(1);
    expect(result.finished).toBe(1);
    expect(result.terminals.find((row) => row.address.terminal_id === terminalId)?.finished).toBe(
      true,
    );
    expect(hub).toBeTruthy();
  });
});

describe("web archive history", () => {
  test("sends the exact runtime target and decodes the stream", async () => {
    const { hub, machine, terminalId, epoch, ops, request } = harness();
    const result = await readArchivedHistory(
      { machine, terminal_id: terminalId, runtime_epoch: epoch },
      ops,
    );
    expect(Buffer.from(result.data).toString()).toBe("hello");
    expect(result.fromSequence).toBe(1);
    expect(result.nextSequence).toBe(2);
    expect(result.truncated).toBe(false);
    expect(request()).toEqual({
      schema_version: 1,
      target: {
        address: { hub_id: hub, terminal_id: terminalId },
        runtime_epoch: epoch,
      },
    });
  });

  test("rejects an unknown machine or invalid size before calling the Hub", async () => {
    const { machine, terminalId, epoch, ops, calls } = harness();
    await expect(
      readArchivedHistory(
        { machine: crypto.randomUUID(), terminal_id: terminalId, runtime_epoch: epoch },
        ops,
      ),
    ).rejects.toThrow("Unknown machine");
    await expect(
      readArchivedHistory(
        { machine, terminal_id: terminalId, runtime_epoch: epoch, max_bytes: 0 },
        ops,
      ),
    ).rejects.toThrow("Invalid history size");
    expect(calls).toEqual([]);
  });

  test("maps Hub failures to actionable statuses", async () => {
    const { machine, terminalId, epoch, ops } = harness();
    ops.history = async () => {
      throw new HubError(-32000, "rejected", { code: "not_found" });
    };
    await expect(
      readArchivedHistory({ machine, terminal_id: terminalId, runtime_epoch: epoch }, ops),
    ).rejects.toMatchObject({ status: 404 });
    ops.history = async () => {
      throw new HubError(-32601, "missing", null);
    };
    await expect(
      readArchivedHistory({ machine, terminal_id: terminalId, runtime_epoch: epoch }, ops),
    ).rejects.toMatchObject({ status: 409 });
  });
});
