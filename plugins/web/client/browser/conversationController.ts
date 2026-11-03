import { api } from "./api.js";
import { byId } from "./dom.js";
import { renderConversation } from "./conversation.js";
import { renderPane, renderSidebar, renderStatus } from "./render.js";
import { fail } from "./requestController.js";
import { dispatch, getState } from "./store.js";
import { getSession } from "./sessionLink.js";
import { selectionPayload } from "./payload.js";
import { lens, selected } from "./appSelectors.js";
import type { Json, Page, Selected } from "./types.js";
import {
  conversationKey,
  type SessionConversationFrame,
  type WireAgentTarget,
} from "../shared/protocol.js";

function watchBinding(target: Selected): WireAgentTarget {
  return selectionPayload(target) as WireAgentTarget;
}

function agentKey(target: Selected): string {
  return conversationKey(watchBinding(target));
}

function applyPage(page: Page): void {
  dispatch({
    type: "conversation/latest",
    entries: page.entries ?? [],
    nextCursor: (page.next_cursor as Json | null) ?? null,
    partial: page.partial ?? [],
  });
  renderConversation(true);
  renderPane();
}

export function frame(message: SessionConversationFrame): void {
  const target = selected(getState());
  if (!target || target.kind !== "agent") return;
  // A keyless error is a session-level target rejection, so it belongs to
  // the active selection rather than a stale actor.
  if (typeof message.key === "string" && message.key !== agentKey(target)) return;
  if (message.type === "conversation") {
    const reading = message.reading as {
      status?: string;
      page?: Page;
    } | null;
    if (reading?.status === "page" && reading.page) applyPage(reading.page);
    else if (reading?.status === "unchanged") {
      dispatch({ type: "conversation/unchanged" });
    }
    return;
  }
  if (message.type === "conversation.error") {
    dispatch({
      type: "conversation/error",
      message: String(message.message ?? "Conversation read failed"),
    });
    renderConversation(false);
    return;
  }
  const reason = String(message.reason ?? "conversation_unavailable");
  if (reason === "history_changed" || reason === "binding_changed") return;
  dispatch({ type: "conversation/unavailable", reason });
  const state = getState();
  if (reason === "unsupported_provider" && state.selection.selected?.kind === "agent") {
    dispatch({ type: "selection/lens", lens: "terminal" });
    renderPane(false);
    syncWatch();
    return;
  }
  renderConversation(false);
  renderPane();
}

/** Start or stop the conversation watch for the current selection and lens. */
export function syncWatch(): void {
  const state = getState();
  const target = selected(state);
  const session = getSession();
  if (!target) {
    session?.unwatchConversation();
    return;
  }
  if (target.kind === "agent" && lens(state) === "chat") {
    session?.watchConversation(watchBinding(target));
  } else {
    session?.unwatchConversation();
  }
}

export async function loadOlder(): Promise<void> {
  const state = getState();
  const target = selected(state);
  if (
    !state.conversation.nextCursor ||
    state.conversation.older ||
    target?.kind !== "agent" ||
    lens(state) !== "chat"
  ) {
    return;
  }
  const cursor = state.conversation.nextCursor;
  dispatch({ type: "conversation/olderStart" });
  renderConversation(false);
  const view = byId("conversation");
  const previousHeight = view.scrollHeight;
  try {
    const result = await api.conversation({
      ...selectionPayload(target),
      cursor,
      since_revision: null,
      since_generation: null,
    });
    if (!result.available || result.reading?.status !== "page" || !result.reading.page) {
      dispatch({ type: "request/status", text: "Older history is unavailable", kind: "bad" });
      return;
    }
    const page = result.reading.page as Page;
    dispatch({
      type: "conversation/olderAppend",
      entries: page.entries ?? [],
      nextCursor: (page.next_cursor as Json | null) ?? null,
      partial: page.partial ?? [],
    });
    renderConversation(false);
    view.scrollTop = view.scrollHeight - previousHeight;
  } catch (error) {
    fail(error);
  } finally {
    dispatch({ type: "conversation/olderEnd" });
    renderConversation(false);
    renderSidebar();
    renderStatus();
  }
}
