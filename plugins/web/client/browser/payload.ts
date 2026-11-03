import type { Agent, Json, Selected, Terminal } from "./types.js";

export function agentPayload(agent: Agent): Json {
  return {
    machine: agent.cursor?.target?.address?.machine,
    agent_id: agent.cursor?.target?.address?.agent_id,
    execution_id: agent.cursor?.target?.execution_id,
    terminal_id: agent.terminal?.address?.terminal_id,
    runtime_epoch: agent.terminal?.runtime_epoch,
  };
}

export function terminalPayload(terminal: Terminal): Json {
  return {
    machine: terminal.address?.machine,
    terminal_id: terminal.address?.terminal_id,
    runtime_epoch: terminal.runtime?.target?.runtime_epoch,
  };
}

export function selectionPayload(selected: Selected | null): Json {
  if (!selected) return {};
  return selected.kind === "agent"
    ? agentPayload(selected.agent)
    : terminalPayload(selected.terminal);
}

/** The Hub, not the browser, is the authority on prompt admission. */
export function canSend(selected: Selected | null): boolean {
  if (!selected) return false;
  if (selected.kind !== "agent") return selected.terminal.runtime?.status === "running";
  const agent = selected.agent;
  return (
    agent.lifecycle?.state === "running" &&
    agent.reachability === "reachable" &&
    agent.freshness === "current" &&
    agent.capabilities?.submit_prompt?.support === "available"
  );
}
