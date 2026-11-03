import { selected } from "./appSelectors.js";
import { getState } from "./store.js";
import { byId } from "./dom.js";
import { renderLifecycle } from "./lifecycle.js";
import { syncTerminalView } from "./terminalMount.js";
import { renderInteraction } from "./interaction.js";
import { renderSidebar } from "./sidebar.js";
import { canSend } from "./payload.js";
import {
  agentContext,
  agentState,
  agentTitle,
  terminalContext,
  terminalOfSelection,
  terminalState,
  terminalTitle,
} from "./model.js";

export { renderSidebar } from "./sidebar.js";

export function renderPane(takeControl = false): void {
  const snapshot = getState();
  const target = selected(snapshot);
  const pending = snapshot.selection.pending;
  const terminal = terminalOfSelection(target);
  const title = !target
    ? pending
      ? `Opening ${pending.kind}…`
      : "No selection"
    : target.kind === "agent"
      ? agentTitle(target.agent)
      : terminalTitle(target.terminal);
  byId("pane-title").textContent = title;
  if (!target) {
    byId("pane-subtitle").textContent = pending
      ? "Waiting for the machine to report it."
      : "Choose an agent or shell on the left.";
  } else {
    const status =
      target.kind === "agent" ? agentState(target.agent) : terminalState(target.terminal);
    const provider =
      target.kind === "agent"
        ? target.agent.provider?.display_name ?? target.agent.provider?.kind
        : null;
    const context =
      target.kind === "agent"
        ? agentContext(target.agent)
        : terminal
          ? terminalContext(target.terminal)
          : "";
    byId("pane-subtitle").textContent = [provider, context, status.label].filter(Boolean).join(" · ");
  }
  const lens = byId("lens");
  lens.hidden = !target || target.kind !== "agent";
  byId("lens-chat").setAttribute("aria-pressed", String(snapshot.selection.lens === "chat"));
  byId("lens-terminal").setAttribute("aria-pressed", String(snapshot.selection.lens === "terminal"));
  renderInteraction(target);
  const chat = target?.kind === "agent" && snapshot.selection.lens === "chat";
  byId("conversation").hidden = !chat;
  byId("terminal-view").hidden = target === null || chat;
  syncTerminalView(target, takeControl);
  byId("empty").hidden = target !== null || pending !== null;
  const composer = byId<HTMLFormElement>("composer");
  composer.hidden = !canSend(target) || !chat;
  byId("chat-locked").hidden = !(target?.kind === "agent" && chat && !canSend(target));
  byId<HTMLTextAreaElement>("prompt").placeholder = "Message the agent…";
  byId<HTMLTextAreaElement>("prompt").disabled = snapshot.request.busy;
  byId<HTMLButtonElement>("send").disabled = snapshot.request.busy;
  renderStatus();
  renderLifecycle();
}

export function renderStatus(): void {
  const status = byId("send-status");
  const current = getState().request.status;
  status.textContent = current.text;
  status.className = `send-status note ${current.kind}`;
}

export function showGate(message = "", kind = ""): void {
  byId("app").hidden = true;
  byId("gate").hidden = false;
  const note = byId("gate-message");
  note.textContent = message || "Read the token from the plugin data directory.";
  note.className = `note ${kind}`;
  setTimeout(() => byId<HTMLInputElement>("token").focus(), 0);
}

export function showApp(): void {
  byId("gate").hidden = true;
  byId("app").hidden = false;
}

export function renderAll(): void {
  renderSidebar();
  renderPane();
  renderStatus();
}
