import type { Agent, Inventory, Terminal } from "./types.js";

export type MachineRow = NonNullable<
  NonNullable<Inventory["hubs"]>["hubs"]
>[number];
export type ProviderRow = NonNullable<MachineRow["providers"]>[number];

export type CatalogueIndex = {
  machines: MachineRow[];
  entryMachine?: string;
  machinesByHandle: Map<string, MachineRow>;
  agents: Agent[];
  agentsById: Map<string, Agent>;
  agentsByTerminal: Map<string, Agent[]>;
  terminals: Terminal[];
  terminalsById: Map<string, Terminal[]>;
  terminalsByAddress: Map<string, Terminal>;
  providers: Map<string, ProviderRow>;
};

const SEP = "\u0000";

export function terminalKey(
  address: { machine?: string; terminal_id?: string } | undefined,
): string {
  return `${address?.machine ?? ""}${SEP}${address?.terminal_id ?? ""}`;
}

export function providerKey(machine: string | undefined, provider: string | undefined): string {
  return `${machine ?? ""}${SEP}${provider ?? ""}`;
}

export function emptyIndex(): CatalogueIndex {
  return {
    machines: [],
    entryMachine: undefined,
    machinesByHandle: new Map(),
    agents: [],
    agentsById: new Map(),
    agentsByTerminal: new Map(),
    terminals: [],
    terminalsById: new Map(),
    terminalsByAddress: new Map(),
    providers: new Map(),
  };
}

function push<T>(map: Map<string, T[]>, key: string, value: T): void {
  const values = map.get(key);
  if (values) values.push(value);
  else map.set(key, [value]);
}

/** One immutable lookup index per catalogue observation. */
export function buildIndex(inventory: Inventory | null): CatalogueIndex {
  if (!inventory) return emptyIndex();
  const index = emptyIndex();
  index.machines = inventory.hubs?.hubs ?? [];
  index.entryMachine = inventory.machine;
  for (const machine of index.machines) {
    if (machine.machine) index.machinesByHandle.set(machine.machine, machine);
    for (const provider of machine.providers ?? []) {
      if (!provider.provider_id) continue;
      const key = providerKey(machine.machine, provider.provider_id);
      if (!index.providers.has(key)) index.providers.set(key, provider);
    }
  }
  index.agents = inventory.agents?.agents ?? [];
  for (const agent of index.agents) {
    const id = agent.cursor?.target?.address?.agent_id;
    if (id && !index.agentsById.has(id)) index.agentsById.set(id, agent);
    const terminal = terminalKey(agent.terminal?.address);
    if (agent.terminal?.address?.terminal_id) push(index.agentsByTerminal, terminal, agent);
  }
  index.terminals = inventory.terminals?.terminals ?? [];
  for (const terminal of index.terminals) {
    const id = terminal.address?.terminal_id;
    if (!id) continue;
    const address = terminalKey(terminal.address);
    if (!index.terminalsByAddress.has(address)) index.terminalsByAddress.set(address, terminal);
    push(index.terminalsById, id, terminal);
  }
  return index;
}
