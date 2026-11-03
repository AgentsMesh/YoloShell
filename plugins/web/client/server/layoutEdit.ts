import { ApiError } from "./errors";
import { admin, hubId, read } from "./hub";
import { invalidateLayout } from "./layoutView";
import { record, uuid } from "./validate";

type Terminal = { hub_id?: string; terminal_id?: string } | null;
export type Pane = { id?: string; terminal?: Terminal };
export type LayoutTree = {
  pane?: Pane;
  split?: { axis?: string; ratio?: number; first?: LayoutTree; second?: LayoutTree };
};
export type Tab = { id?: string; title?: string; root?: LayoutTree };
type LayoutAddress = { hub_id: string; layout_id: string };
type Document = { address?: LayoutAddress; revision?: number; tabs?: Tab[] };

export type LayoutEditOps = {
  get(address: unknown): Promise<unknown>;
  update(change: unknown, destination: string): Promise<unknown>;
};

const live: LayoutEditOps = {
  get: (address) => read(hubId(), (hub) => hub.getLayout(address)),
  update: (change, destination) => admin(destination, (hub) => hub.updateLayout(change, destination)),
};

function split(tree: LayoutTree, paneId: string, axis: string): [LayoutTree, boolean] {
  if (tree.pane?.id === paneId) {
    return [
      {
        split: {
          axis,
          ratio: 50,
          first: { pane: tree.pane },
          second: { pane: { id: crypto.randomUUID(), terminal: null } },
        },
      },
      true,
    ];
  }
  if (!tree.split) return [tree, false];
  const [first, changed] = split(tree.split.first ?? {}, paneId, axis);
  if (changed) return [{ split: { ...tree.split, first } }, true];
  const [second, changedSecond] = split(tree.split.second ?? {}, paneId, axis);
  return [{ split: { ...tree.split, second } }, changedSecond];
}

function close(tree: LayoutTree, paneId: string): [LayoutTree | null, boolean] {
  if (tree.pane?.id === paneId) return [null, true];
  if (!tree.split) return [tree, false];
  const [first, changed] = close(tree.split.first ?? {}, paneId);
  if (changed) return [first ?? tree.split.second ?? {}, true];
  const [second, changedSecond] = close(tree.split.second ?? {}, paneId);
  return [second ?? tree.split.first ?? {}, changedSecond];
}

export function splitPane(tabs: Tab[], paneId: string, axis: string): [Tab[], boolean] {
  let changed = false;
  const next = tabs.map((tab) => {
    const [root, hit] = split(tab.root ?? {}, paneId, axis);
    changed ||= hit;
    return hit ? { ...tab, root } : tab;
  });
  return [next, changed];
}

export function closePane(tabs: Tab[], paneId: string): [Tab[], boolean] {
  let changed = false;
  const next = tabs.map((tab) => {
    const [root, hit] = close(tab.root ?? {}, paneId);
    changed ||= hit;
    if (!hit) return tab;
    // A tab always keeps one pane, even when its last bound terminal is closed.
    return { ...tab, root: root ?? { pane: { id: crypto.randomUUID(), terminal: null } } };
  });
  return [next, changed];
}

function address(value: unknown): LayoutAddress {
  const item = record(value);
  return { hub_id: uuid(item.hub_id), layout_id: uuid(item.layout_id) };
}

export async function editLayout(
  body: Record<string, unknown>,
  ops: LayoutEditOps = live,
): Promise<unknown> {
  const item = record(body);
  const paneId = uuid(item.pane_id);
  const layoutAddress = address(item.address);
  const reply = (await ops.get(layoutAddress)) as { document?: Document | null };
  const document = reply.document;
  if (!document?.tabs || document.revision === undefined) {
    throw new ApiError(409, "The shared layout no longer exists");
  }
  const [tabs, changed] =
    item.operation === "split"
      ? splitPane(document.tabs, paneId, item.axis === "vertical" ? "vertical" : "horizontal")
      : item.operation === "close"
        ? closePane(document.tabs, paneId)
        : [document.tabs, false];
  if (!changed) throw new ApiError(409, "The pane is no longer in the layout");
  const change = {
    address: layoutAddress,
    request_id: crypto.randomUUID(),
    expected_revision: document.revision,
    tabs,
  };
  const updated = (await ops.update(change, layoutAddress.hub_id)) as {
    document?: Document | null;
  };
  invalidateLayout();
  const next = updated.document;
  if (!next || JSON.stringify(next.tabs) !== JSON.stringify(tabs)) {
    throw new ApiError(409, "Another client changed the layout; reloading");
  }
  return {
    layout: { address: next.address, revision: next.revision, tabs: next.tabs },
  };
}
