import { filter } from "./appSelectors.js";
import { terminalKey } from "./catalogueIndex.js";
import { getState } from "./store.js";
import type { Agent, Json, Selected, Terminal } from "./types.js";

export function terminalOf(agent: Agent): Terminal | undefined {
  return getState().catalogue.index.terminalsByAddress.get(
    terminalKey(agent.terminal?.address),
  );
}

export function terminalOfSelection(selected: Selected | null): Terminal | undefined {
  if (!selected) return undefined;
  return selected.kind === "agent" ? terminalOf(selected.agent) : selected.terminal;
}

export function providerName(kind: string): string {
  switch (kind.toLowerCase()) {
    case "codex":
      return "Codex";
    case "claude_code":
      return "Claude Code";
    case "qwen_code":
      return "Qwen Code";
    case "gemini":
      return "Gemini";
    case "grok":
      return "Grok";
    default:
      return kind.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
  }
}

export function providerMarkKind(kind: string): string {
  const value = kind.toLowerCase();
  if (value.includes("claude")) return "C";
  if (value.includes("codex")) return "X";
  if (value.includes("qwen")) return "Q";
  if (value.includes("gemini")) return "G";
  return (providerName(kind)[0] ?? "A").toUpperCase();
}

function providerLabel(agent: Agent): string {
  return providerName(agent.provider?.display_name?.trim() || agent.provider?.kind || "agent");
}

const GENERIC_TERMINAL_NAMES = new Set(["bash", "sh", "zsh", "fish", "dash", "terminal"]);

export function agentTitle(agent: Agent): string {
  const terminal = terminalOf(agent);
  if (terminal) {
    const name = terminal.name?.trim();
    if (name && !GENERIC_TERMINAL_NAMES.has(name.toLowerCase())) return name;
    const title = terminalTitle(terminal);
    if (
      title &&
      !GENERIC_TERMINAL_NAMES.has(title.toLowerCase()) &&
      !/^[^@\s]+@[^:\s]+:/.test(title)
    ) {
      return title;
    }
  }
  return providerLabel(agent);
}

export function agentContext(agent: Agent): string {
  const path = shortPath(agent.working_directory ?? terminalOf(agent)?.cwd ?? "");
  const name = terminalOf(agent)?.name?.trim() || (agent.terminal?.address?.terminal_id ? `#${agent.terminal.address.terminal_id.slice(0, 4)}` : "terminal");
  const hubs = getState().catalogue.index.machines;
  const origin = agent.cursor?.target?.address?.machine;
  const machine = hubs.length > 1
    ? hubs.find((hub) => hub.machine === origin)?.label
    : undefined;
  const title = agentTitle(agent);
  return [machine, path, name !== title ? name : ""].filter(Boolean).join(" · ");
}

export function terminalTitle(terminal: Terminal): string {
  const name = terminal.name?.trim();
  const dynamic = (terminal.presentation?.title ?? "")
    .replace(/^[\u2800-\u28ff\s]+/u, "")
    .trim();
  const cwdName = (terminal.cwd ?? "").split("/").filter(Boolean).at(-1) ?? "";
  let title = (dynamic || name || "Terminal").replace(/^[^@\s]+@[^:\s]+:\s*/, "");
  if (title.startsWith("/")) title = title.split("/").filter(Boolean).at(-1) ?? title;
  return name && (title === cwdName || title === "") ? name : title;
}
export function terminalContext(terminal: Terminal): string {
  const name = terminal.name?.trim();
  const path = shortPath(terminal.cwd ?? "");
  if (!name || name === terminalTitle(terminal)) return path;
  return [name, path].filter(Boolean).join(" · ");
}

export function providerMark(agent: Agent): string {
  return providerMarkKind(agent.provider?.kind ?? providerLabel(agent));
}

export function shortPath(path: string): string {
  if (!path) return "";
  const parts = path.split("/").filter(Boolean);
  return parts.length <= 2 ? path : `…/${parts.slice(-2).join("/")}`;
}

export function agentState(agent: Agent): { key: string; label: string } {
  const lifecycle = agent.lifecycle?.state ?? "unknown";
  if (lifecycle === "starting") return { key: "working", label: "Starting" };
  if (lifecycle === "exited") return { key: "exited", label: "Exited" };
  if (lifecycle !== "running") return { key: "unknown", label: "Unknown" };
  if (agent.reachability === "unreachable") return { key: "blocked", label: "Unreachable" };
  switch (agent.activity) {
    case "working":
      return { key: "working", label: "Working" };
    case "waiting_for_input":
      return { key: "input", label: "Needs input" };
    case "waiting_for_approval":
      return { key: "blocked", label: "Approval" };
    case "idle":
      return { key: "idle", label: "Idle" };
    default:
      return { key: "unknown", label: "Running" };
  }
}

export function terminalState(terminal: Terminal): { key: string; label: string } {
  switch (terminal.runtime?.status) {
    case "starting":
      return { key: "working", label: "Starting" };
    case "running":
      return { key: "done", label: "Running" };
    case "output_blocked":
      return { key: "blocked", label: "Output blocked" };
    case "finalizing":
      return { key: "working", label: "Finalizing" };
    case "exited":
      return { key: "exited", label: "Exited" };
    case "lost":
      return { key: "blocked", label: "Lost" };
    case "unreachable":
      return { key: "blocked", label: "Unreachable" };
    default:
      return { key: "unknown", label: "Unknown" };
  }
}

function agentRank(agent: Agent): number {
  const lifecycle = agent.lifecycle?.state;
  if (lifecycle === "running") return 0;
  if (lifecycle === "starting") return 1;
  if (lifecycle === "exited") return 2;
  return 3;
}

function activityRank(agent: Agent): number {
  return ["working", "waiting_for_input", "waiting_for_approval", "idle"].indexOf(
    agent.activity ?? "",
  );
}

export function sortAgents(agents: Agent[]): Agent[] {
  return [...agents].sort(
    (left, right) =>
      agentRank(left) - agentRank(right) ||
      activityRank(left) - activityRank(right) ||
      agentTitle(left).localeCompare(agentTitle(right)),
  );
}

export function sortTerminals(terminals: Terminal[]): Terminal[] {
  const rank = (terminal: Terminal) =>
    terminal.runtime?.status === "running" ? 0 : terminal.runtime?.status === "lost" ? 2 : 1;
  return [...terminals].sort(
    (left, right) => rank(left) - rank(right) || terminalTitle(left).localeCompare(terminalTitle(right)),
  );
}

export function matches(text: string): boolean {
  const value = filter(getState()).trim();
  return value === "" || text.toLowerCase().includes(value.toLowerCase());
}

