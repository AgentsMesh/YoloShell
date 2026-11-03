import { ApiError } from "./errors";
import { entryMachine, labelFor, machineFor } from "./machineHandles";

type Capability = { create?: boolean; agent_start?: boolean };

function rows(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new ApiError(502, `The ${label} catalogue is missing`);
  }
  return value;
}

/**
 * A missing section is a failed observation, never an empty catalogue. The
 * browser must be able to distinguish "nothing exists" from "we could not
 * read it", otherwise one degraded reply erases every row on screen.
 */
export function catalogue(raw: unknown): {
  terminals: unknown[];
  agents: unknown[];
  hubs: unknown[];
} {
  const item = (typeof raw === "object" && raw !== null ? raw : {}) as {
    terminals?: { terminals?: unknown };
    agents?: { agents?: unknown };
    hubs?: { hubs?: unknown };
  };
  return {
    terminals: rows(item.terminals?.terminals, "terminal"),
    agents: rows(item.agents?.agents, "agent"),
    hubs: rows(item.hubs?.hubs, "hub"),
  };
}

function machineAddress(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return value;
  const address = value as Record<string, unknown>;
  const hub = address.hub_id;
  return typeof hub === "string"
    ? { ...address, machine: machineFor(hub) }
    : address;
}

function machineAgent(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return value;
  const agent = value as Record<string, any>;
  const target = agent.cursor?.target ?? {};
  const terminal = agent.terminal ?? {};
  return {
    ...agent,
    cursor: { ...agent.cursor, target: { ...target, address: machineAddress(target.address) } },
    terminal: { ...terminal, address: machineAddress(terminal.address) },
  };
}

function machineTerminal(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return value;
  const terminal = value as Record<string, any>;
  return { ...terminal, address: machineAddress(terminal.address) };
}

function machineHub(value: unknown, capabilities?: Map<string, Capability>): unknown {
  if (typeof value !== "object" || value === null) return value;
  const hub = value as Record<string, any>;
  if (typeof hub.hub_id !== "string") return hub;
  return {
    ...hub,
    machine: machineFor(hub.hub_id),
    label: labelFor(hub.hub_id, hub.label),
    capabilities: capabilities?.get(hub.hub_id) ?? {},
  };
}

/** The browser-facing catalogue carries opaque machine handles, never Hub ids. */
export function machineCatalogue(
  raw: unknown,
  caps?: Map<string, Capability>,
) {
  const lists = catalogue(raw);
  return {
    machine: entryMachine(),
    terminals: { terminals: lists.terminals.map(machineTerminal) },
    agents: { agents: lists.agents.map(machineAgent) },
    hubs: { hubs: lists.hubs.map((hub) => machineHub(hub, caps)) },
  };
}

