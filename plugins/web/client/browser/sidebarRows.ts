import type { AppState } from "./appState.js";
import {
  activeAgent,
  activeTerminal,
  filter,
  inScope,
  inventory,
  pendingAgents,
  selected,
} from "./appSelectors.js";
import { byId, el } from "./dom.js";
import {
  agentContext,
  agentState,
  agentTitle,
  matches,
  providerMark,
  sortAgents,
  sortTerminals,
  terminalContext,
  terminalState,
  terminalTitle,
} from "./model.js";
import type { PendingAgent } from "./pendingAgents.js";
import { pendingRow } from "./pendingRows.js";
import { row } from "./rows.js";
import type { Agent, Terminal } from "./types.js";

function agentSelected(agent: Agent, state: AppState): boolean {
  const target = selected(state);
  if (target?.kind !== "agent") return false;
  return (
    target.agent.cursor?.target?.address?.machine ===
      agent.cursor?.target?.address?.machine &&
    target.agent.cursor?.target?.address?.agent_id ===
      agent.cursor?.target?.address?.agent_id &&
    target.agent.cursor?.target?.execution_id ===
      agent.cursor?.target?.execution_id
  );
}

function terminalSelected(terminal: Terminal, state: AppState): boolean {
  const target = selected(state);
  return (
    target?.kind === "terminal" &&
    target.terminal.address?.machine === terminal.address?.machine &&
    target.terminal.address?.terminal_id === terminal.address?.terminal_id
  );
}

function emptyRow(text: string): HTMLLIElement {
  return el("li", "empty-state", text);
}

function appendRows(list: HTMLElement, rows: HTMLLIElement[]): void {
  list.replaceChildren(...rows);
}

export type SidebarRows = {
  allAgents: Agent[];
  agents: Agent[];
  terminals: Terminal[];
  pending: PendingAgent[];
};

export function renderSidebarRows(snapshot: AppState): SidebarRows {
  const hubs = inventory(snapshot)?.hubs?.hubs ?? [];
  const rawAgents = inventory(snapshot)?.agents?.agents ?? [];
  const allTerminals = inventory(snapshot)?.terminals?.terminals ?? [];
  const liveTerminals = allTerminals.filter(activeTerminal);
  const pending = pendingAgents(snapshot);
  const allAgents = sortAgents(
    rawAgents.filter((agent) => activeAgent(agent, snapshot)),
  );
  const agents = allAgents.filter((agent) =>
    inScope(agent.cursor?.target?.address?.machine, snapshot),
  );
  const agentTerminals = new Set(
    agents.map(
      (agent) =>
        `${agent.terminal?.address?.machine}:${agent.terminal?.address?.terminal_id}`,
    ),
  );
  const pendingTerminals = new Set(
    pending.map((agent) => `${agent.machine}:${agent.terminalId}`),
  );
  const terminals = sortTerminals(liveTerminals).filter((terminal) => {
    const key = `${terminal.address?.machine}:${terminal.address?.terminal_id}`;
    return (
      inScope(terminal.address?.machine, snapshot) &&
      !agentTerminals.has(key) &&
      !pendingTerminals.has(key)
    );
  });
  const pendingRows: HTMLLIElement[] = [];
  for (const item of pending) {
    if (!inScope(item.machine, snapshot)) continue;
    const terminal = liveTerminals.find(
      (candidate) =>
        candidate.address?.machine === item.machine &&
        candidate.address?.terminal_id === item.terminalId,
    );
    const entry = pendingRow(
      item,
      terminal,
      terminal ? terminalSelected(terminal, snapshot) : false,
    );
    if (entry) pendingRows.push(entry);
  }
  const agentRows: HTMLLIElement[] = [];
  for (const agent of agents) {
    const context = agentContext(agent);
    if (!matches(`${agentTitle(agent)} ${context} ${agent.provider?.display_name ?? ""}`)) continue;
    agentRows.push(
      row(
        "agent",
        agent.cursor?.target?.address?.agent_id ?? "",
        agent.cursor?.target?.address?.machine ?? "",
        agentTitle(agent),
        context,
        providerMark(agent),
        agentState(agent),
        agentSelected(agent, snapshot),
      ),
    );
  }
  const terminalRows: HTMLLIElement[] = [];
  for (const terminal of terminals) {
    const context = terminalContext(terminal);
    if (!matches(`${terminalTitle(terminal)} ${context} ${terminal.name ?? ""}`)) continue;
    terminalRows.push(
      row(
        "terminal",
        terminal.address?.terminal_id ?? "",
        terminal.address?.machine ?? "",
        terminalTitle(terminal),
        context,
        "T",
        terminalState(terminal),
        terminalSelected(terminal, snapshot),
      ),
    );
  }
  const filtering = filter(snapshot).trim() !== "";
  const agentUnsupported =
    hubs.length > 0 && hubs.every((hub) => hub.capabilities?.agent_start === false);
  const visibleAgents = [...pendingRows, ...agentRows];
  appendRows(
    byId("agent-list"),
    visibleAgents.length
      ? visibleAgents
      : [
          emptyRow(
            filtering
              ? "No matching agents"
              : agentUnsupported
                ? "No agents · machines are shell-only"
                : "No active agents",
          ),
        ],
  );
  appendRows(
    byId("terminal-list"),
    terminalRows.length
      ? terminalRows
      : [emptyRow(filtering ? "No matching shells" : "No running shells")],
  );
  byId("agent-count").textContent = String(visibleAgents.length);
  byId("terminal-count").textContent = String(terminalRows.length);
  return { allAgents, agents, terminals, pending };
}
