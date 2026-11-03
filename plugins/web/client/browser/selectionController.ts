import { renderConversation } from "./conversation.js";
import { syncWatch } from "./conversationController.js";
import { closeDrawer } from "./drawer.js";
import { renderPane, renderSidebar } from "./render.js";
import { rememberSelection, rememberedSelection } from "./selectionIntent.js";
import { dispatch, getState } from "./store.js";
import {
  activeAgent,
  activeTerminal,
  inScope,
  inventory,
  lens,
  pendingSelection,
  selected,
  selectedForIntent,
} from "./appSelectors.js";
import type { SelectionIntent } from "./types.js";

function afterSelection(takeControl: boolean, remember: boolean): void {
  const state = getState();
  const target = selected(state);
  if (target && remember) {
    rememberSelection(
      target.kind,
      target.kind === "agent"
        ? (target.agent.cursor?.target?.address?.agent_id ?? "")
        : (target.terminal.address?.terminal_id ?? ""),
      target.kind === "agent"
        ? target.agent.cursor?.target?.address?.machine
        : target.terminal.address?.machine,
    );
  }
  closeDrawer();
  renderSidebar();
  renderPane(takeControl);
  if (lens(state) === "chat") renderConversation();
  syncWatch();
}

export function choose(
  kind: string,
  id: string,
  takeControl = true,
  machine?: string,
): void {
  const before = getState();
  const intent: SelectionIntent = {
    kind: kind === "agent" ? "agent" : "terminal",
    id,
    machine,
    take: takeControl,
  };
  const next = dispatch({ type: "selection/choose", intent });
  if (next === before) return;
  afterSelection(takeControl, true);
}

/**
 * Select a created terminal before the catalogue has reported it. The
 * creation result names the terminal; waiting for the projection used to
 * leave the previous selection actionable for up to the refresh deadline.
 */
export function chooseCreated(machine: string, id: string): void {
  const before = getState();
  const intent: SelectionIntent = { kind: "terminal", id, machine, take: true };
  const next = dispatch({ type: "selection/created", intent });
  if (next === before) return;
  if (next.selection.selected) {
    afterSelection(true, true);
    return;
  }
  rememberSelection("terminal", id, machine);
  closeDrawer();
  renderSidebar();
  renderPane(false);
  syncWatch();
}

export function restoreSelection(): boolean {
  const value = rememberedSelection();
  if (!value) return false;
  choose(value.kind, value.id, false, value.machine);
  if (!selected(getState())) choose(value.kind, value.id, false);
  return selected(getState()) !== null;
}

/** Prefer a plain shell, then an agent, within the selected machine scope. */
export function autoSelect(): void {
  const state = getState();
  const catalogue = inventory(state);
  const terminals = (catalogue?.terminals?.terminals ?? [])
    .filter(activeTerminal)
    .filter((terminal) => inScope(terminal.address?.machine, state));
  const shell = terminals.find((terminal) => !terminal.agent) ?? terminals[0];
  if (shell?.address?.terminal_id) {
    choose("terminal", shell.address.terminal_id, false, shell.address.machine);
    return;
  }
  const agent = (catalogue?.agents?.agents ?? []).find(
    (candidate) =>
      activeAgent(candidate, state) &&
      inScope(candidate.cursor?.target?.address?.machine, state),
  );
  if (agent?.cursor?.target?.address?.agent_id) {
    choose(
      "agent",
      agent.cursor.target.address.agent_id,
      false,
      agent.cursor.target.address.machine,
    );
  }
}

export function setLens(nextLens: "chat" | "terminal"): void {
  const state = getState();
  if (state.selection.selected?.kind !== "agent") return;
  if (state.selection.lens === nextLens) {
    if (nextLens === "terminal") renderPane(false);
    return;
  }
  dispatch({ type: "selection/lens", lens: nextLens });
  renderPane(false);
  if (nextLens === "chat") renderConversation();
  syncWatch();
}

/** Adopt a held selection once the catalogue reports it. */
export function resolvePendingSelection(): SelectionIntent | null {
  const state = getState();
  const intent = pendingSelection(state);
  if (!intent) return null;
  const target = selectedForIntent(state, intent);
  if (!target) return null;
  dispatch({ type: "selection/adopt", intent });
  afterSelection(intent.take === true, false);
  return intent;
}
