import { describe, expect, test } from "bun:test";
import type { archive, read } from "../server/hub";
import { machineFor } from "../server/machineHandles";
import { archiveOps } from "../server/archiveSource";

function source(methods: string[]) {
  const calls: { method: string; destination: string }[] = [];
  const client = {
    request: async (method: string, _params: unknown, destination: string) => {
      calls.push({ method, destination });
      return { hello: { methods } };
    },
    listArchived: async (destination: string) => {
      calls.push({ method: "list", destination });
      return { terminals: [{ archived: true }] };
    },
    readTerminalHistory: async (_request: unknown, destination: string) => {
      calls.push({ method: "history", destination });
      return { history: {} };
    },
  };
  const at = async (
    _hub: string,
    operation: (value: typeof client) => Promise<unknown>,
  ) => operation(client);
  const peer = crypto.randomUUID();
  const ops = archiveOps(
    at as unknown as typeof read,
    at as unknown as typeof archive,
    () => [{ hub: peer, machine: machineFor(peer), label: "Peer", reachable: true }],
    () => null,
  );
  return { calls, ops, peer };
}

describe("archive Hub source", () => {
  test("addresses every peer call at that peer instead of the entry Hub", async () => {
    const { calls, ops, peer } = source(["read_terminal_history"]);
    expect(await ops.list(peer)).toEqual({
      terminals: [{ archived: true }],
      supports: true,
    });
    await ops.history(peer, { target: {} });
    expect(calls).toEqual([
      { method: "hello", destination: peer },
      { method: "list", destination: peer },
      { method: "history", destination: peer },
    ]);
  });

  test("detects an unsupported peer before listing its archive", async () => {
    const { calls, ops, peer } = source([]);
    expect(await ops.list(peer)).toEqual({ terminals: [], supports: false });
    expect(calls).toEqual([{ method: "hello", destination: peer }]);
  });
});
