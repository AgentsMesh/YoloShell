import { describe, expect, test } from "bun:test";
import { machineFor } from "../server/machineHandles";
import { layout } from "../server/layoutView";

const ADDRESS = { hub_id: crypto.randomUUID(), layout_id: crypto.randomUUID() };
const FIRST = { machine: crypto.randomUUID(), terminal_id: crypto.randomUUID() };
const SECOND = { machine: crypto.randomUUID(), terminal_id: crypto.randomUUID() };

describe("shared layout", () => {
  test("reads the first published layout document", async () => {
    const result = await layout({
      list: async () => ({ documents: [{ address: ADDRESS, revision: 3 }] }),
      get: async () => ({
        document: { address: ADDRESS, revision: 3, tabs: [{ title: "work", root: {} }] },
      }),
    });
    expect(result).toEqual({
      layout: {
        address: { ...ADDRESS, machine: machineFor(ADDRESS.hub_id) },
        revision: 3,
        tabs: [{ title: "work", root: {} }],
      },
    });
  });

  test("reports no layout when the Hub has none", async () => {
    expect(await layout({ list: async () => ({ documents: [] }), get: async () => ({}) })).toEqual({
      layout: null,
    });
  });
});

describe("layout editing", () => {
  test("splits a pane and closes it back to its sibling", async () => {
    const { splitPane, closePane } = await import("../server/layoutEdit");
    const pane = { id: crypto.randomUUID(), terminal: FIRST };
    const [split, changed] = splitPane([{ title: "work", root: { pane } }], pane.id!, "horizontal");
    expect(changed).toBe(true);
    const root = split[0].root as any;
    expect(root.split.axis).toBe("horizontal");
    expect(root.split.first.pane.terminal).toEqual(FIRST);
    expect(root.split.second.pane.terminal).toBeNull();
    const [closed, removed] = closePane(split, root.split.first.pane.id);
    expect(removed).toBe(true);
    expect((closed[0].root as any).pane.id).toBe(root.split.second.pane.id);
  });

  test("a closed last pane leaves one empty pane", async () => {
    const { closePane } = await import("../server/layoutEdit");
    const pane = { id: crypto.randomUUID(), terminal: FIRST };
    const [closed] = closePane([{ title: "work", root: { pane } }], pane.id!);
    expect((closed[0].root as any).pane.terminal).toBeNull();
  });

  test("submits a CAS update and rejects a conflicting document", async () => {
    const { editLayout } = await import("../server/layoutEdit");
    const pane = { id: crypto.randomUUID(), terminal: FIRST };
    const document = {
      address: ADDRESS,
      revision: 4,
      tabs: [{ title: "work", root: { pane } }],
    };
    let change: any = null;
    const result = await editLayout(
      { address: ADDRESS, operation: "split", pane_id: pane.id },
      {
        get: async () => ({ document }),
        update: async (value) => {
          change = value;
          return { document: { ...document, revision: 5, tabs: value.tabs } };
        },
      },
    );
    expect(change.expected_revision).toBe(4);
    expect((change.tabs[0].root as any).split).toBeDefined();
    expect((result as any).layout.revision).toBe(5);
    await expect(
      editLayout(
        { address: ADDRESS, operation: "close", pane_id: pane.id },
        {
          get: async () => ({ document }),
          update: async () => ({ document: { ...document, revision: 6 } }),
        },
      ),
    ).rejects.toThrow();
  });
});
