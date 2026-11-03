import type { AppState } from "./appState.js";
import type { Intent } from "./appIntents.js";

type Busy = Extract<Intent, { type: "request/busy" }>;
type Status = Extract<Intent, { type: "request/status" }>;
type Notify = Extract<Intent, { type: "attention/notify" }>;
type Quiet = Extract<Intent, { type: "attention/quiet" }>;

export function busy(state: AppState, intent: Busy): AppState {
  return state.request.busy === intent.busy
    ? state
    : { ...state, request: { ...state.request, busy: intent.busy } };
}

export function status(state: AppState, intent: Status): AppState {
  return {
    ...state,
    request: {
      ...state.request,
      status: { text: intent.text, kind: intent.kind ?? "" },
    },
  };
}

export function notify(state: AppState, intent: Notify): AppState {
  return state.attention.notify === intent.value
    ? state
    : { ...state, attention: { ...state.attention, notify: intent.value } };
}

export function quiet(state: AppState, intent: Quiet): AppState {
  return state.attention.quiet === intent.value
    ? state
    : { ...state, attention: { ...state.attention, quiet: intent.value } };
}
