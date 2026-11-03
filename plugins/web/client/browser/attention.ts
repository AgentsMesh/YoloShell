import { providerFeatures } from "./appSelectors.js";
import { byId } from "./dom.js";
import { agentContext, agentState, agentTitle, providerMark } from "./model.js";
import { row } from "./rows.js";
import { getState } from "./store.js";
import type { Agent } from "./types.js";

/** Agents that need the user: approval, an answer, or a finished turn. */
export function attentionAgents(agents: Agent[]): Agent[] {
  return agents.filter(
    (agent) =>
      agent.activity === "waiting_for_approval" ||
      agent.activity === "waiting_for_input" ||
      answerable(agent),
  );
}

/** Whether the agent exposes a typed interaction the Web can answer now. */
export function answerable(agent: Agent): boolean {
  return (
    agent.capabilities?.respond_interaction?.support === "available" &&
    agent.interaction?.reference?.id !== undefined
  );
}

/** A provider prompt that is owned by the native terminal, not a typed card. */
export function terminalApproval(agent: Agent): boolean {
  if (agent.activity !== "waiting_for_approval" || answerable(agent)) return false;
  const features = providerFeatures(
    getState(),
    agent.cursor?.target?.address?.machine,
    agent.provider?.kind,
  );
  return features?.interaction !== "typed";
}

/** Agents whose current approval or interaction should appear under Needs you. */
export function needsAgents(agents: Agent[]): Agent[] {
  return agents.filter((agent) => answerable(agent) || terminalApproval(agent));
}

export function renderAttention(agents: Agent[]): void {
  const needs = needsAgents(agents);
  byId("needs").hidden = needs.length === 0;
  byId("needs-count").textContent = String(needs.length);
  byId("needs-list").replaceChildren(
    ...needs.map((agent) => {
      const item = row(
        "agent",
        agent.cursor?.target?.address?.agent_id ?? "",
        agent.cursor?.target?.address?.machine ?? "",
        agentTitle(agent),
        agentContext(agent),
        providerMark(agent),
        agentState(agent),
        false,
      );
      if (terminalApproval(agent)) {
        item.querySelector("button")?.setAttribute("data-lens", "terminal");
      }
      return item;
    }),
  );
}
