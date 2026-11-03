import { initialState, type AppState } from "./appState.js";
import { reduce } from "./appReducer.js";
import type { Intent } from "./appIntents.js";
import { loadPendingAgents } from "./pendingAgents.js";
import { loadNotify, loadQuiet } from "./preferences.js";

export type Listener = (next: AppState, previous: AppState) => void;

let current = createState();
const listeners = new Set<Listener>();

function createState(): AppState {
  return initialState({
    quiet: loadQuiet(),
    notify: loadNotify(),
    pendingAgents: loadPendingAgents(),
  });
}

export function getState(): AppState {
  return current;
}

/** The only way to change application state. */
export function dispatch(intent: Intent): AppState {
  const previous = current;
  const next = reduce(previous, intent);
  if (next === previous) return previous;
  current = next;
  for (const listener of [...listeners]) listener(next, previous);
  return next;
}

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Reset the store from persisted preferences; used by tests and a fresh token. */
export function resetStore(): AppState {
  const previous = current;
  current = createState();
  for (const listener of [...listeners]) listener(current, previous);
  return current;
}
