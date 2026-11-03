import type { AppState } from "./appState.js";
import type { Intent } from "./appIntents.js";
import {
  connected,
  disconnected,
  filter,
  observe,
  patch,
  pendingAgents,
  replace,
  scope,
} from "./catalogueReducer.js";
import {
  latest,
  olderAppend,
  olderEnd,
  olderStart,
  failure,
  unchanged,
  unavailable,
} from "./conversationReducer.js";
import {
  busy,
  notify,
  quiet,
  status,
} from "./requestReducer.js";
import {
  adopt,
  choose,
  created,
  interactionCleared,
  lens,
} from "./selectionReducer.js";

export function reduce(state: AppState, intent: Intent): AppState {
  switch (intent.type) {
    case "catalogue/observe":
      return observe(state, intent);
    case "catalogue/replace":
      return replace(state, intent);
    case "catalogue/patch":
      return patch(state, intent);
    case "catalogue/connected":
      return connected(state);
    case "catalogue/disconnected":
      return disconnected(state);
    case "catalogue/pendingAgents":
      return pendingAgents(state, intent);
    case "ui/scope":
      return scope(state, intent);
    case "ui/filter":
      return filter(state, intent);
    case "selection/choose":
      return choose(state, intent);
    case "selection/created":
      return created(state, intent);
    case "selection/adopt":
      return adopt(state, intent);
    case "selection/lens":
      return lens(state, intent);
    case "selection/interactionCleared":
      return interactionCleared(state);
    case "conversation/latest":
      return latest(state, intent);
    case "conversation/unchanged":
      return unchanged(state);
    case "conversation/unavailable":
      return unavailable(state, intent);
    case "conversation/error":
      return failure(state, intent);
    case "conversation/olderStart":
      return olderStart(state);
    case "conversation/olderAppend":
      return olderAppend(state, intent);
    case "conversation/olderEnd":
      return olderEnd(state);
    case "request/busy":
      return busy(state, intent);
    case "request/status":
      return status(state, intent);
    case "attention/notify":
      return notify(state, intent);
    case "attention/quiet":
      return quiet(state, intent);
  }
  return state;
}
