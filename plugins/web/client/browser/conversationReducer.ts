import type { AppState, ConversationState } from "./appState.js";
import type { Intent } from "./appIntents.js";
import { entryKey } from "./conversationModel.js";

const PARTIAL_NOTE = "Some history is unavailable";

type Latest = Extract<Intent, { type: "conversation/latest" }>;
type OlderAppend = Extract<Intent, { type: "conversation/olderAppend" }>;
type Unavailable = Extract<Intent, { type: "conversation/unavailable" }>;
type ErrorIntent = Extract<Intent, { type: "conversation/error" }>;

function note(partial: string[]): string {
  return partial.length ? PARTIAL_NOTE : "";
}

function page(
  state: AppState,
  entries: ConversationState["entries"],
  nextCursor: ConversationState["nextCursor"],
  partial: string[],
): ConversationState {
  return {
    ...state.conversation,
    entries,
    nextCursor,
    available: true,
    reason: "",
    note: note(partial),
    loaded: true,
  };
}

export function latest(state: AppState, intent: Latest): AppState {
  return {
    ...state,
    conversation: page(state, intent.entries, intent.nextCursor, intent.partial),
  };
}

export function unchanged(state: AppState): AppState {
  return {
    ...state,
    conversation: {
      ...state.conversation,
      available: true,
      loaded: true,
    },
  };
}

export function unavailable(state: AppState, intent: Unavailable): AppState {
  return {
    ...state,
    conversation: {
      ...state.conversation,
      available: false,
      reason: intent.reason,
    },
  };
}

export function failure(state: AppState, intent: ErrorIntent): AppState {
  return {
    ...state,
    conversation: { ...state.conversation, note: intent.message },
  };
}

export function olderStart(state: AppState): AppState {
  return state.conversation.older
    ? state
    : { ...state, conversation: { ...state.conversation, older: true } };
}

export function olderAppend(state: AppState, intent: OlderAppend): AppState {
  const present = new Set(state.conversation.entries.map(entryKey));
  return {
    ...state,
    conversation: {
      ...state.conversation,
      entries: [
        ...intent.entries.filter((entry) => !present.has(entryKey(entry))),
        ...state.conversation.entries,
      ],
      nextCursor: intent.nextCursor,
      note: note(intent.partial),
      loaded: true,
    },
  };
}

export function olderEnd(state: AppState): AppState {
  return state.conversation.older
    ? { ...state, conversation: { ...state.conversation, older: false } }
    : state;
}
