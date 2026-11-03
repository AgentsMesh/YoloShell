import {
  liveTerminal,
  selectableTerminal,
  visibleAgent,
} from "./catalogue.js";
import { providerKey, terminalKey, type ProviderRow } from "./catalogueIndex.js";
import type { AppState, ConversationState, SelectionState } from "./appState.js";
import type {
  Agent,
  Inventory,
  Lens,
  Selected,
  SelectionId,
  SelectionIntent,
  Terminal,
} from "./types.js";

export function inventory(state: AppState): Inventory | null {
  return state.catalogue.inventory;
}

export function pendingAgents(state: AppState) {
  return state.creation.pendingAgents;
}

export function scope(state: AppState): string | null {
  return state.ui.scope;
}

export function filter(state: AppState): string {
  return state.ui.filter;
}

export function selection(state: AppState): SelectionState {
  return state.selection;
}

export function selectionId(state: AppState): SelectionId | null {
  return state.selection.selected;
}

export function pendingSelection(state: AppState): SelectionIntent | null {
  return state.selection.pending;
}

export function lens(state: AppState): Lens {
  return state.selection.lens;
}

export function epoch(state: AppState): number {
  return state.selection.epoch;
}

export function conversation(state: AppState): ConversationState {
  return state.conversation;
}

export function busy(state: AppState): boolean {
  return state.request.busy;
}

export function status(state: AppState) {
  return state.request.status;
}

export function quiet(state: AppState): boolean {
  return state.attention.quiet;
}

export function notify(state: AppState): boolean {
  return state.attention.notify;
}

export function agentById(
  id: string | undefined,
  state: AppState,
): Agent | undefined {
  return id ? state.catalogue.index.agentsById.get(id) : undefined;
}

export function terminalById(
  id: string | undefined,
  machine: string | undefined,
  state: AppState,
): Terminal | undefined {
  if (!id) return undefined;
  if (machine !== undefined) {
    return state.catalogue.index.terminalsByAddress.get(
      terminalKey({ machine, terminal_id: id }),
    );
  }
  return state.catalogue.index.terminalsById.get(id)?.[0];
}

export function selectedForIntent(
  state: AppState,
  intent: SelectionIntent,
): Selected | null {
  if (intent.kind === "agent") {
    const agent = agentById(intent.id, state);
    return agent ? { kind: "agent", agent } : null;
  }
  const terminal = terminalById(intent.id, intent.machine, state);
  if (!terminal || !selectableTerminal(terminal)) return null;
  return { kind: "terminal", terminal };
}

export function selectedForId(state: AppState, id: SelectionId): Selected | null {
  return selectedForIntent(state, { kind: id.kind, id: id.id, machine: id.machine });
}

/** Resolve the current selection identity against the live catalogue. */
export function selected(state: AppState): Selected | null {
  const id = selectionId(state);
  return id ? selectedForId(state, id) : null;
}

export function selectionKeyOf(selected: Selected | null): string {
  if (!selected) return "";
  if (selected.kind === "agent") {
    const target = selected.agent.cursor?.target;
    return `agent:${target?.address?.agent_id ?? ""}:${target?.execution_id ?? ""}`;
  }
  return `terminal:${selected.terminal.address?.machine ?? ""}:${selected.terminal.address?.terminal_id ?? ""}`;
}

export function selectionKey(state: AppState): string {
  return selectionKeyOf(selected(state));
}

/** Stable key for a stored identity, using its resolvable row when present. */
export function identityKey(state: AppState, id: SelectionId | null): string {
  return id ? selectionKeyOf(selectedForId(state, id)) : "";
}

export function selectionOrigin(state: AppState): string | undefined {
  const id = selectionId(state);
  if (id?.machine !== undefined) return id.machine;
  const value = selected(state);
  if (!value) return undefined;
  return value.kind === "agent"
    ? value.agent.cursor?.target?.address?.machine
    : value.terminal.address?.machine;
}

export function activeAgent(agent: Agent, state: AppState): boolean {
  return visibleAgent(agent, state.catalogue.index.terminals);
}

export function activeTerminal(terminal: Terminal): boolean {
  return liveTerminal(terminal);
}

export function inScope(machine: string | undefined, state: AppState): boolean {
  const value = scope(state);
  return value === null || machine === value;
}

export function providerFeatures(
  state: AppState,
  machine: string | undefined,
  providerId: string | undefined,
): ProviderRow["features"] {
  return state.catalogue.index.providers.get(providerKey(machine, providerId))?.features;
}
