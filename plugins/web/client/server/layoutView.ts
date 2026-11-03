import { hubId, read } from "./hub";
import { machineFor } from "./machineHandles";
import { SnapshotCache } from "./snapshotCache";

type LayoutSummary = { address?: unknown; revision?: number };
export type LayoutOps = {
  list(): Promise<unknown>;
  get(address: unknown): Promise<unknown>;
};

export const layoutOps: LayoutOps = {
  list: () => read(hubId(), (hub) => hub.listLayouts()),
  get: (address) => read(hubId(), (hub) => hub.getLayout(address)),
};

const cache = new SnapshotCache<unknown>(2_000);

function layoutTree(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return value;
  const tree = value as Record<string, any>;
  if (tree.pane) {
    const terminal = tree.pane.terminal;
    const address =
      terminal && typeof terminal === "object" && typeof terminal.hub_id === "string"
        ? { ...terminal, machine: machineFor(terminal.hub_id) }
        : terminal;
    return { ...tree, pane: { ...tree.pane, terminal: address } };
  }
  if (!tree.split) return tree;
  return {
    ...tree,
    split: {
      ...tree.split,
      first: layoutTree(tree.split.first),
      second: layoutTree(tree.split.second),
    },
  };
}

function layoutAddress(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return value;
  const address = value as Record<string, unknown>;
  return typeof address.hub_id === "string"
    ? { ...address, machine: machineFor(address.hub_id) }
    : address;
}

function machineLayout(document: {
  address?: unknown;
  revision?: number;
  tabs?: unknown[];
}) {
  return {
    address: layoutAddress(document.address),
    revision: document.revision,
    tabs: (document.tabs ?? []).map((value) => {
      const tab = value as Record<string, any>;
      return { ...tab, root: layoutTree(tab.root) };
    }),
  };
}

async function load(ops: LayoutOps): Promise<unknown> {
  const listed = (await ops.list()) as { documents?: LayoutSummary[] };
  const summary = listed.documents?.[0];
  if (!summary?.address) return { layout: null };
  const got = (await ops.get(summary.address)) as {
    document?: { address?: unknown; revision?: number; tabs?: unknown[] } | null;
  };
  const document = got.document;
  if (!document) return { layout: null };
  return { layout: machineLayout(document) };
}

/** The first published shared layout, if the entry Hub has one. */
export function layout(ops: LayoutOps = layoutOps): Promise<unknown> {
  if (ops !== layoutOps) return load(ops);
  return cache.get(() => load(layoutOps));
}

/** A layout write must not leave a pre-write document in the poll cache. */
export function invalidateLayout(): void {
  cache.clear();
}
