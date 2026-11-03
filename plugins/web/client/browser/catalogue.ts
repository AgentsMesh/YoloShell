import type { Agent, Inventory, Snapshot, Terminal } from "./types.js";

/**
 * Visibility is a runtime-phase question, not a reachability question. A row
 * that is starting or winding down still exists; hiding it would let one
 * poll's transitional observation erase it from the screen.
 */
const LIVE_TERMINAL = new Set(["starting", "running", "output_blocked", "finalizing"]);
const FINISHED_TERMINAL = new Set(["exited", "lost"]);

export function liveTerminal(terminal: Terminal): boolean {
  return LIVE_TERMINAL.has(terminal.runtime?.status ?? "");
}

export function finishedTerminal(terminal: Terminal): boolean {
  return FINISHED_TERMINAL.has(terminal.runtime?.status ?? "");
}

export function selectableTerminal(terminal: Terminal): boolean {
  return liveTerminal(terminal) || finishedTerminal(terminal);
}

export function liveAgent(agent: Agent): boolean {
  const state = agent.lifecycle?.state;
  return state === "starting" || state === "running";
}

/**
 * A running agent stays visible while its terminal is live, even when one
 * observation says it is unreachable. A transient "unknown" lifecycle is the
 * same kind of observation gap, but it may only keep a row alive while the
 * terminal is live and the agent is still reachable; otherwise retired
 * entries with no terminal would become permanent ghosts.
 */
export function visibleAgent(agent: Agent, terminals: Terminal[]): boolean {
  const state = agent.lifecycle?.state;
  const id = agent.terminal?.address?.terminal_id;
  const terminal = id
    ? terminals.find((candidate) => candidate.address?.terminal_id === id)
    : undefined;
  if (state === "starting" || state === "running") {
    if (!terminal) {
      return agent.reachability === "reachable" && agent.freshness === "current";
    }
    return liveTerminal(terminal);
  }
  return (
    state === "unknown" &&
    terminal !== undefined &&
    liveTerminal(terminal) &&
    agent.reachability === "reachable"
  );
}

export function hasRows(inventory: Inventory | null): boolean {
  return (
    (inventory?.terminals?.terminals?.length ?? 0) > 0 ||
    (inventory?.agents?.agents?.length ?? 0) > 0
  );
}

/** A reply is usable only when every catalogue section is present. */
export function coherent(value: unknown): value is Inventory {
  if (typeof value !== "object" || value === null) return false;
  const item = value as Inventory;
  return (
    Array.isArray(item.terminals?.terminals) &&
    Array.isArray(item.agents?.agents) &&
    Array.isArray(item.hubs?.hubs)
  );
}

function newer(previous: Snapshot | undefined, next: Snapshot | undefined): boolean {
  if (!next || typeof next.sequence !== "number") return true;
  if (!previous || previous.server !== next.server) return true;
  return next.sequence > (previous.sequence ?? -1);
}

export type Application = { apply: boolean; emptyStreak: number };

/**
 * Decide whether a response may replace the visible catalogue. Out-of-order
 * replies are ignored by the server's snapshot sequence. A first empty reply
 * after a populated catalogue is treated as a degraded observation and
 * confirmed by the following poll; an authoritative empty catalogue arrives
 * one poll later instead of flashing both lists empty.
 */
export function application(
  previous: Inventory | null,
  applied: Snapshot | undefined,
  next: Inventory,
  emptyStreak: number,
): Application {
  if (!newer(applied, next.snapshot)) return { apply: false, emptyStreak };
  if (!hasRows(next)) {
    if (hasRows(previous) && emptyStreak < 1) {
      return { apply: false, emptyStreak: emptyStreak + 1 };
    }
    return { apply: true, emptyStreak: emptyStreak + 1 };
  }
  return { apply: true, emptyStreak: 0 };
}
