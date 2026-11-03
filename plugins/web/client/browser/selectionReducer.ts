import { emptyConversation, type AppState } from "./appState.js";
import type { Intent } from "./appIntents.js";
import { selectedForIntent } from "./appSelectors.js";
import { buildIndex } from "./catalogueIndex.js";
import type { SelectionId, SelectionIntent } from "./types.js";

type Choose = Extract<Intent, { type: "selection/choose" }>;
type Created = Extract<Intent, { type: "selection/created" }>;
type Adopt = Extract<Intent, { type: "selection/adopt" }>;
type Lens = Extract<Intent, { type: "selection/lens" }>;

function identity(intent: SelectionIntent): SelectionId {
  return { kind: intent.kind, id: intent.id, machine: intent.machine };
}

function chooseSelection(state: AppState, id: SelectionId): AppState {
  return {
    ...state,
    selection: {
      selected: id,
      pending: null,
      lens: id.kind === "agent" ? "chat" : "terminal",
      epoch: state.selection.epoch + 1,
    },
    conversation: emptyConversation(),
    request: { ...state.request, status: { text: "", kind: "" } },
  };
}

export function choose(state: AppState, intent: Choose): AppState {
  const target = selectedForIntent(state, intent.intent);
  return target ? chooseSelection(state, identity(intent.intent)) : state;
}

export function created(state: AppState, intent: Created): AppState {
  const target = selectedForIntent(state, intent.intent);
  if (target) return chooseSelection(state, identity(intent.intent));
  return {
    ...state,
    selection: {
      ...state.selection,
      selected: null,
      pending: intent.intent,
      epoch: state.selection.epoch + 1,
    },
    conversation: emptyConversation(),
    request: { ...state.request, status: { text: "", kind: "" } },
  };
}

export function adopt(state: AppState, intent: Adopt): AppState {
  if (!selectedForIntent(state, intent.intent)) return state;
  return {
    ...state,
    selection: {
      selected: identity(intent.intent),
      pending: null,
      lens: intent.intent.kind === "agent" ? "chat" : "terminal",
      epoch: state.selection.epoch + 1,
    },
    conversation: emptyConversation(),
  };
}

export function lens(state: AppState, intent: Lens): AppState {
  if (state.selection.selected?.kind !== "agent") return state;
  if (state.selection.lens === intent.lens) return state;
  return {
    ...state,
    selection: {
      ...state.selection,
      lens: intent.lens,
      epoch: state.selection.epoch + 1,
    },
    request: { ...state.request, status: { text: "", kind: "" } },
  };
}

export function interactionCleared(state: AppState): AppState {
  const id = state.selection.selected;
  if (id?.kind !== "agent") return state;
  const agent = state.catalogue.index.agentsById.get(id.id);
  if (!agent?.interaction) return state;
  const inventory = state.catalogue.inventory;
  if (!inventory) return state;
  const agents = (inventory.agents?.agents ?? []).map((candidate) =>
    candidate === agent ? { ...candidate, interaction: null } : candidate,
  );
  const next = {
    ...inventory,
    agents: { ...(inventory.agents ?? {}), agents },
  };
  return {
    ...state,
    catalogue: { ...state.catalogue, inventory: next, index: buildIndex(next) },
  };
}
