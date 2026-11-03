import { attentionAgents } from "./attention.js";
import { byId, el } from "./dom.js";
import { agentState, agentTitle } from "./model.js";
import { getState } from "./store.js";
import type { Agent } from "./types.js";

const MAX_ALERTS = 3;
const LIFETIME_MS = 12_000;
let shown = new Set<string>();
let seeded = false;

function attentionKey(agent: Agent): string {
  const id = agent.cursor?.target?.address?.agent_id ?? "";
  const kind =
    agent.activity === "waiting_for_approval"
      ? "approval"
      : agent.activity === "waiting_for_input"
        ? "input"
        : `interaction:${agent.interaction?.reference?.id ?? ""}`;
  return `${id}:${kind}`;
}

function toast(agent: Agent): void {
  const host = byId("alerts");
  const id = agent.cursor?.target?.address?.agent_id;
  if (!host || !id) return;
  const item = el("div", "alert");
  const open = el("button", "alert-open");
  open.type = "button";
  open.dataset.kind = "agent";
  open.dataset.id = id;
  open.dataset.machine = agent.cursor?.target?.address?.machine ?? "";
  open.append(
    el("strong", "", agentTitle(agent)),
    el("span", "", agentState(agent).label),
  );
  const close = el("button", "alert-close", "×");
  close.type = "button";
  close.setAttribute("aria-label", "Dismiss notification");
  close.addEventListener("click", () => item.remove());
  item.append(open, close);
  host.append(item);
  while (host.childElementCount > MAX_ALERTS) host.firstElementChild?.remove();
  setTimeout(() => item.remove(), LIFETIME_MS);
}

/**
 * Announce agents that newly need the user. Quiet or Notify off only
 * re-baselines, so re-enabling never replays the current backlog.
 */
export function alertNeeds(agents: Agent[]): void {
  const currentAgents = attentionAgents(agents);
  const current = new Set(currentAgents.map(attentionKey));
  const state = getState();
  if (seeded && state.attention.notify && !state.attention.quiet) {
    for (const agent of currentAgents) {
      if (!shown.has(attentionKey(agent))) toast(agent);
    }
  }
  shown = current;
  seeded = true;
}

export function clearAlerts(): void {
  byId("alerts")?.replaceChildren();
}
