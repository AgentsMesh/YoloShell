import { labelFor, machineFor } from "./machineHandles";
import type { ProviderFeatures } from "../shared/protocol";

export type { ProviderFeatures } from "../shared/protocol";

export type MachineCapabilities = {
  create?: boolean;
  agent_start?: boolean;
  terminal_lifecycle?: boolean;
};
export type ProviderOption = {
  provider_id: string;
  display_name: string;
  available: boolean;
  features?: ProviderFeatures;
};
export type MachineView = {
  machine: string;
  label: string;
  entry: boolean;
  reachable: boolean;
  freshness: "live" | "stale" | "unreachable";
  epoch: string;
  revision: number;
  capabilities: MachineCapabilities;
  providers: ProviderOption[];
  terminals: unknown[];
  agents: unknown[];
};

function address(value: unknown, hub: string, key: "terminal_id" | "agent_id") {
  if (typeof value !== "object" || value === null) return value;
  const item = value as Record<string, unknown>;
  const id = item[key];
  if (typeof id !== "string") return { machine: machineFor(hub) };
  return { machine: machineFor(hub), [key]: id };
}

export function terminalView(value: unknown, hub: string): unknown {
  if (typeof value !== "object" || value === null) return value;
  const item = value as Record<string, any>;
  return {
    ...item,
    address: address(item.address, hub, "terminal_id"),
    agent: item.agent
      ? {
          ...item.agent,
          address: item.agent.address?.agent_id
            ? { agent_id: item.agent.address.agent_id }
            : undefined,
        }
      : null,
    runtime: item.runtime
      ? {
          ...item.runtime,
          target: item.runtime.target
            ? {
                ...item.runtime.target,
                address: address(item.runtime.target.address, hub, "terminal_id"),
              }
            : undefined,
        }
      : item.runtime,
  };
}

function agentView(value: unknown, hub: string): unknown {
  if (typeof value !== "object" || value === null) return value;
  const item = value as Record<string, any>;
  const target = item.cursor?.target ?? {};
  return {
    ...item,
    cursor: {
      ...item.cursor,
      target: {
        ...target,
        address: address(target.address, hub, "agent_id"),
      },
    },
    terminal: item.terminal
      ? {
          ...item.terminal,
          address: address(item.terminal.address, hub, "terminal_id"),
        }
      : item.terminal,
  };
}

export function machineView(input: {
  hub: string;
  label?: string;
  entry: boolean;
  reachable: boolean;
  freshness: MachineView["freshness"];
  epoch: string;
  revision: number;
  methods: string[];
  providers?: ProviderOption[];
  terminals: unknown[];
  agents: unknown[];
}): MachineView {
  const methods = new Set(input.methods);
  return {
    machine: machineFor(input.hub),
    label: labelFor(input.hub, input.label),
    entry: input.entry,
    reachable: input.reachable,
    freshness: input.freshness,
    epoch: input.epoch,
    revision: input.revision,
    capabilities: {
      create: methods.has("create"),
      agent_start: methods.has("agent_start"),
      terminal_lifecycle: methods.has("terminal"),
    },
    providers: input.providers ?? [],
    terminals: input.terminals.map((value) => terminalView(value, input.hub)),
    agents: input.agents.map((value) => agentView(value, input.hub)),
  };
}
