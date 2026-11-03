import { describe, expect, test } from "bun:test";
import { collectPages } from "../../sdk/src/pages";

describe("paged catalogue reads", () => {
  test("walks next cursors unchanged and keeps every page", async () => {
    const seen: unknown[] = [];
    const pages: Record<string, unknown>[] = [
      { agents: [{ id: 1 }], next: { address: { agent_id: "a" }, execution_id: "e" } },
      { agents: [{ id: 2 }], next: "second" },
      { agents: [{ id: 3 }], next: null },
    ];
    const rows = await collectPages("agents", async (after) => {
      seen.push(after);
      return pages[seen.length - 1];
    });
    expect(rows).toEqual([{ id: 1 }, { id: 2 }, { id: 3 }]);
    expect(seen).toEqual([
      null,
      { address: { agent_id: "a" }, execution_id: "e" },
      "second",
    ]);
  });

  test("ends on a repeated cursor instead of looping", async () => {
    let calls = 0;
    const cursor = { hub_id: "h" };
    const rows = await collectPages("hubs", async () => {
      calls += 1;
      return { hubs: calls === 1 ? [{ hub_id: "h", reachable: true }] : [], next: cursor };
    });
    expect(rows).toEqual([{ hub_id: "h", reachable: true }]);
    expect(calls).toBe(2);
  });

  test("ignores a malformed page and bounds a walk with no end", async () => {
    let calls = 0;
    const malformed = await collectPages("terminals", async () => {
      calls += 1;
      return { terminals: "not-a-list", next: null };
    });
    expect(malformed).toEqual([]);
    expect(calls).toBe(1);

    calls = 0;
    await collectPages("terminals", async () => {
      calls += 1;
      return { terminals: [], next: calls };
    });
    expect(calls).toBe(64);
  });
});
