import { emptyIndex, type CatalogueIndex } from "./catalogueIndex.js";
import type { PendingAgent } from "./pendingAgents.js";
import type {
  Entry,
  Inventory,
  Json,
  Lens,
  SelectionId,
  SelectionIntent,
  Snapshot,
} from "./types.js";

export type Status = { text: string; kind: string };

export type CatalogueState = {
  inventory: Inventory | null;
  index: CatalogueIndex;
  snapshot?: Snapshot;
  emptyStreak: number;
  connected: boolean;
  updatedAt: number | null;
};

export type UiState = { scope: string | null; filter: string };

export type CreationState = { pendingAgents: PendingAgent[] };

export type SelectionState = {
  selected: SelectionId | null;
  pending: SelectionIntent | null;
  lens: Lens;
  /** Selection/lens fence for stale prompt confirmations. */
  epoch: number;
};

export type ConversationState = {
  entries: Entry[];
  nextCursor: Json | null;
  available: boolean;
  reason: string;
  note: string;
  loaded: boolean;
  older: boolean;
};

export type RequestState = { busy: boolean; status: Status };

export type AttentionState = { quiet: boolean; notify: boolean };

export type AppState = {
  catalogue: CatalogueState;
  ui: UiState;
  creation: CreationState;
  selection: SelectionState;
  conversation: ConversationState;
  request: RequestState;
  attention: AttentionState;
};

export function emptyConversation(): ConversationState {
  return {
    entries: [],
    nextCursor: null,
    available: true,
    reason: "",
    note: "",
    loaded: false,
    older: false,
  };
}

export function initialState(input: {
  quiet: boolean;
  notify: boolean;
  pendingAgents: PendingAgent[];
}): AppState {
  return {
    catalogue: {
      inventory: null,
      index: emptyIndex(),
      snapshot: undefined,
      emptyStreak: 0,
      connected: false,
      updatedAt: null,
    },
    ui: { scope: null, filter: "" },
    creation: { pendingAgents: input.pendingAgents },
    selection: { selected: null, pending: null, lens: "terminal", epoch: 0 },
    conversation: emptyConversation(),
    request: { busy: false, status: { text: "", kind: "" } },
    attention: { quiet: input.quiet, notify: input.notify },
  };
}
