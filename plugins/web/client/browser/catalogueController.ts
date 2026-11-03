import { application, coherent } from "./catalogue.js";
import { applyCataloguePatch, type CataloguePatch } from "./cataloguePatch.js";
import { syncWatch } from "./conversationController.js";
import { renderPane, renderSidebar } from "./render.js";
import {
  autoSelect,
  resolvePendingSelection,
  restoreSelection,
} from "./selectionController.js";
import { dispatch, getState } from "./store.js";
import { getSession } from "./sessionLink.js";
import {
  inventory,
  selectionKey,
} from "./appSelectors.js";
import {
  addPendingAgent,
  prunePendingAgents,
  savePendingAgents,
} from "./pendingAgents.js";
import type { Inventory } from "./types.js";

let resyncing = false;

function prunePending(): void {
  const state = getState();
  const pruned = prunePendingAgents(
    state.creation.pendingAgents,
    inventory(state)?.terminals?.terminals ?? [],
    inventory(state)?.agents?.agents ?? [],
  );
  if (pruned.length === state.creation.pendingAgents.length) return;
  savePendingAgents(pruned);
  dispatch({ type: "catalogue/pendingAgents", pendingAgents: pruned });
}

function afterCatalogue(first: boolean, beforeKey: string): void {
  resyncing = false;
  const resolved = resolvePendingSelection();
  prunePending();
  renderSidebar();
  renderPane(resolved?.take === true);
  const state = getState();
  if (!state.selection.selected) {
    if (state.selection.pending) return;
    if (!(first && restoreSelection())) autoSelect();
    return;
  }
  if (beforeKey !== selectionKey(state)) syncWatch();
}

function accepted(): string {
  return selectionKey(getState());
}

export function applyInventory(next: unknown): void {
  if (!coherent(next)) return;
  const state = getState();
  const first = state.catalogue.inventory === null;
  const beforeKey = accepted();
  const decision = application(
    state.catalogue.inventory,
    state.catalogue.snapshot,
    next,
    state.catalogue.emptyStreak,
  );
  const at = Date.now();
  if (decision.apply) {
    dispatch({
      type: "catalogue/replace",
      inventory: next,
      snapshot: next.snapshot,
      emptyStreak: decision.emptyStreak,
      at,
    });
  } else {
    dispatch({ type: "catalogue/observe", emptyStreak: decision.emptyStreak, at });
    return;
  }
  afterCatalogue(first, beforeKey);
}

export function applyPatch(raw: unknown): void {
  const state = getState();
  const beforeKey = accepted();
  const result = applyCataloguePatch(state.catalogue.inventory, raw as CataloguePatch);
  if (!result.applied) {
    requestSnapshot();
    return;
  }
  dispatch({ type: "catalogue/patch", inventory: result.inventory as Inventory, at: Date.now() });
  afterCatalogue(false, beforeKey);
}

export function connected(): void {
  dispatch({ type: "catalogue/connected" });
}

export function disconnected(): void {
  dispatch({ type: "catalogue/disconnected" });
  renderSidebar();
}

export function requestSnapshot(): void {
  if (resyncing) return;
  resyncing = true;
  getSession()?.snapshot();
}

export function refresh(): void {
  getSession()?.refresh();
}

export function scopeChanged(scope: string | null): void {
  dispatch({ type: "ui/scope", scope });
  renderSidebar();
  if (!getState().selection.selected) autoSelect();
  renderPane();
}

export function filterChanged(value: string): void {
  dispatch({ type: "ui/filter", filter: value });
  renderSidebar();
}

export function notePendingAgent(
  machine: string,
  terminalId: string,
  provider: string,
  name?: string,
  cwd?: string,
): void {
  const state = getState();
  const pendingAgents = addPendingAgent(state.creation.pendingAgents, {
    machine,
    terminalId,
    provider,
    name,
    cwd,
  });
  savePendingAgents(pendingAgents);
  dispatch({ type: "catalogue/pendingAgents", pendingAgents });
  renderSidebar();
}
