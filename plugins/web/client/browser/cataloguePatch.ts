import type { Agent, Inventory, Snapshot, Terminal } from "./types.js";

type RowSet<T> = { upsert?: T[]; remove?: string[] };

export type CataloguePatch = {
  machine?: string;
  epoch?: string;
  base_revision?: number;
  revision?: number;
  hub?: Record<string, any>;
  snapshot?: Snapshot;
  terminals?: RowSet<Terminal>;
  agents?: RowSet<Agent>;
};

export type PatchResult =
  | { applied: true; inventory: Inventory }
  | { applied: false; inventory: Inventory | null };

function terminalKey(row: Terminal): string | null {
  const value = row.address?.terminal_id;
  return typeof value === "string" && value ? value : null;
}

function agentKey(row: Agent): string | null {
  const value = row.cursor?.target?.address?.agent_id;
  return typeof value === "string" && value ? value : null;
}

function applyRows<T>(
  rows: T[],
  patch: RowSet<T> | undefined,
  keyOf: (row: T) => string | null,
): T[] | null {
  if (!patch) return rows;
  const indexed = new Map<string, T>();
  for (const row of rows) {
    const key = keyOf(row);
    if (!key) return null;
    indexed.set(key, row);
  }
  for (const row of patch.upsert ?? []) {
    const key = keyOf(row);
    if (!key) return null;
    indexed.set(key, row);
  }
  for (const key of patch.remove ?? []) indexed.delete(key);
  return [...indexed.values()];
}

function newer(inventory: Inventory, snapshot: Snapshot | undefined): boolean {
  if (!snapshot || typeof snapshot.server !== "string" || typeof snapshot.sequence !== "number") {
    return false;
  }
  const current = inventory.snapshot;
  if (!current || typeof current.server !== "string" || typeof current.sequence !== "number") {
    return true;
  }
  if (current.server !== snapshot.server) return false;
  return snapshot.sequence > current.sequence;
}

/**
 * Apply one machine delta. Any gap, epoch change, stale sequence or malformed
 * key rejects the patch; the caller then asks for a full snapshot.
 */
export function applyCataloguePatch(
  inventory: Inventory | null,
  patch: CataloguePatch,
): PatchResult {
  if (!inventory) return { applied: false, inventory: null };
  const hubs = inventory.hubs?.hubs;
  const terminals = inventory.terminals?.terminals;
  const agents = inventory.agents?.agents;
  if (!Array.isArray(hubs) || !Array.isArray(terminals) || !Array.isArray(agents)) {
    return { applied: false, inventory };
  }
  if (
    typeof patch.machine !== "string" ||
    typeof patch.epoch !== "string" ||
    typeof patch.base_revision !== "number" ||
    typeof patch.revision !== "number" ||
    patch.revision <= patch.base_revision ||
    !patch.hub ||
    patch.hub.machine !== patch.machine ||
    patch.hub.epoch !== patch.epoch ||
    !newer(inventory, patch.snapshot)
  ) {
    return { applied: false, inventory };
  }
  const index = hubs.findIndex((hub) => hub.machine === patch.machine);
  if (index < 0) return { applied: false, inventory };
  const hub = hubs[index];
  if (hub.epoch !== patch.epoch || hub.revision !== patch.base_revision) {
    return { applied: false, inventory };
  }
  const nextTerminals = applyRows(terminals, patch.terminals, terminalKey);
  const nextAgents = applyRows(agents, patch.agents, agentKey);
  if (!nextTerminals || !nextAgents) return { applied: false, inventory };
  const nextHubs = hubs.slice();
  nextHubs[index] = {
    ...patch.hub,
    machine: patch.machine,
    epoch: patch.epoch,
    revision: patch.revision,
  };
  return {
    applied: true,
    inventory: {
      ...inventory,
      snapshot: patch.snapshot,
      hubs: { ...inventory.hubs, hubs: nextHubs },
      terminals: { ...inventory.terminals, terminals: nextTerminals },
      agents: { ...inventory.agents, agents: nextAgents },
    },
  };
}
