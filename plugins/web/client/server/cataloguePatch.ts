import type { MachineView, ProviderOption } from "./machineView";

export type MachineHub = {
  machine: string;
  label: string;
  entry: boolean;
  reachable: boolean;
  freshness: MachineView["freshness"];
  epoch: string;
  revision: number;
  capabilities: MachineView["capabilities"];
  providers: ProviderOption[];
};

export type RowDelta = { upsert: unknown[]; remove: string[] };

export type MachinePatch = {
  machine: string;
  epoch: string;
  base_revision: number;
  revision: number;
  hub: MachineHub;
  terminals: RowDelta;
  agents: RowDelta;
};

export type PatchDecision =
  | { kind: "none" }
  | { kind: "snapshot" }
  | { kind: "patch"; patch: MachinePatch };

export function hubSummary(view: MachineView): MachineHub {
  return {
    machine: view.machine,
    label: view.label,
    entry: view.entry,
    reachable: view.reachable,
    freshness: view.freshness,
    epoch: view.epoch,
    revision: view.revision,
    capabilities: view.capabilities,
    providers: view.providers ?? [],
  };
}

function id(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function terminalKey(value: unknown): string | null {
  return id((value as { address?: { terminal_id?: unknown } } | null)?.address?.terminal_id);
}

export function agentKey(value: unknown): string | null {
  return id(
    (value as { cursor?: { target?: { address?: { agent_id?: unknown } } } } | null)?.cursor
      ?.target?.address?.agent_id,
  );
}

type Indexed = Map<string, { row: unknown; json: string }>;

function index(rows: unknown[], keyOf: (value: unknown) => string | null): Indexed | null {
  const result: Indexed = new Map();
  for (const row of rows) {
    const key = keyOf(row);
    if (!key) return null;
    result.set(key, { row, json: JSON.stringify(row) });
  }
  return result;
}

function delta(
  previous: unknown[],
  next: unknown[],
  keyOf: (value: unknown) => string | null,
): RowDelta | null {
  const before = index(previous, keyOf);
  const after = index(next, keyOf);
  if (!before || !after) return null;
  const upsert: unknown[] = [];
  for (const [key, value] of after) {
    if (before.get(key)?.json !== value.json) upsert.push(value.row);
  }
  const remove = [...before.keys()].filter((key) => !after.has(key));
  return { upsert, remove };
}

function changed(previous: MachineView, next: MachineView): boolean {
  return (
    previous.label !== next.label ||
    previous.entry !== next.entry ||
    previous.reachable !== next.reachable ||
    previous.freshness !== next.freshness ||
    JSON.stringify(previous.capabilities) !== JSON.stringify(next.capabilities) ||
    JSON.stringify(previous.providers) !== JSON.stringify(next.providers)
  );
}

/** Decide whether one machine view is a patch, a forced snapshot, or no change. */
export function machinePatch(previous: MachineView, next: MachineView): PatchDecision {
  if (previous.machine !== next.machine || previous.epoch !== next.epoch) {
    return { kind: "snapshot" };
  }
  if (next.revision <= previous.revision) return { kind: "snapshot" };
  const terminals = delta(previous.terminals, next.terminals, terminalKey);
  const agents = delta(previous.agents, next.agents, agentKey);
  if (!terminals || !agents) return { kind: "snapshot" };
  if (
    !changed(previous, next) &&
    terminals.upsert.length === 0 &&
    terminals.remove.length === 0 &&
    agents.upsert.length === 0 &&
    agents.remove.length === 0
  ) {
    return { kind: "none" };
  }
  return {
    kind: "patch",
    patch: {
      machine: next.machine,
      epoch: next.epoch,
      base_revision: previous.revision,
      revision: next.revision,
      hub: hubSummary(next),
      terminals,
      agents,
    },
  };
}
