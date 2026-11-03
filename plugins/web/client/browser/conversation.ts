import { getState } from "./store.js";
import { renderMarkdown } from "./markdownRender.js";
import { byId, el } from "./dom.js";
import { entryLabel, partText, timeLabel } from "./conversationModel.js";
import type { Entry, Part } from "./types.js";

function reasonLabel(reason: string): string {
  switch (reason) {
    case "evidence_unavailable":
      return "no readable transcript for this agent";
    case "reader_busy":
      return "the conversation reader is busy";
    case "history_limit_exceeded":
      return "the transcript exceeds the reader limit";
    case "unsupported_provider":
      return "this provider has no conversation reader";
    default:
      return reason.replaceAll("_", " ");
  }
}

function partView(part: Part): HTMLElement {
  if (part.kind === "text") {
    const text = el("div", "part text markdown");
    text.append(renderMarkdown(part.text ?? ""));
    return text;
  }
  if (part.kind === "thinking") {
    const details = el("details", "part thinking");
    details.append(el("summary", "", "Thinking"), el("pre", "", part.text ?? ""));
    return details;
  }
  if (part.kind === "tool_call") {
    const details = el("details", "part tool");
    details.append(
      el("summary", "", `Tool · ${part.name ?? "call"}`),
      el("pre", "", part.arguments ?? ""),
    );
    return details;
  }
  if (part.kind === "tool_result") {
    const details = el("details", "part tool");
    details.append(
      el("summary", "", `Result${part.is_error ? " · error" : ""}${part.call_id ? ` · ${part.call_id}` : ""}`),
      el("pre", "", part.output ?? ""),
    );
    return details;
  }
  if (part.kind === "compaction") {
    return el("div", "part notice", `Context compaction: ${part.summary ?? ""}`);
  }
  if (part.kind === "image_unavailable") {
    return el("div", "part notice", `Image unavailable${part.media_type ? ` (${part.media_type})` : ""}`);
  }
  if (part.kind === "unsupported") {
    return el("div", "part notice", `Unsupported record: ${part.native_kind ?? "unknown"}`);
  }
  return el("div", "part notice", partText(part));
}

function turnView(entry: Entry): HTMLElement {
  const article = el("article", `turn turn-role-${entry.role ?? "runtime"}`);
  const head = el("div", "turn-head");
  head.append(el("span", "", entryLabel(entry)));
  const time = timeLabel(entry.timestamp);
  if (time) head.append(el("span", "", time));
  article.append(head);
  for (const part of entry.parts ?? []) article.append(partView(part));
  return article;
}

export function renderConversation(stick = true): void {
  const view = byId("conversation");
  const nearBottom = view.scrollTop + view.clientHeight >= view.scrollHeight - 48;
  const turns = byId("turns");
  const snapshot = getState();
  const conversation = snapshot.conversation;
  const entries = conversation.entries;
  const hadTurns = turns.childElementCount > 0;
  turns.replaceChildren(...entries.map(turnView));
  const note = byId("conversation-note");
  note.className = "note";
  if (!conversation.available) {
    const reason = conversation.reason
      ? `: ${reasonLabel(conversation.reason)}`
      : "";
    const restart =
      conversation.reason === "evidence_unavailable"
        ? " If this agent started before the current runtime, restart it to enable Chat."
        : "";
    note.textContent = `Conversation unavailable${reason}. Use Terminal to interact with this agent.${restart}`;
    note.classList.add("bad");
  } else if (conversation.note) {
    note.textContent = conversation.note;
  } else if (!conversation.loaded) {
    note.textContent = "Loading conversation…";
  } else if (entries.length === 0) {
    note.textContent = "No recorded messages yet.";
  } else {
    note.textContent = "";
  }
  note.hidden = note.textContent === "";
  const older = byId<HTMLButtonElement>("older");
  older.hidden = !snapshot.conversation.nextCursor || snapshot.conversation.older;
  older.disabled = snapshot.conversation.older;
  older.textContent = snapshot.conversation.older ? "Loading…" : "Load older messages";
  if (stick && (nearBottom || !hadTurns)) view.scrollTop = view.scrollHeight;
}
